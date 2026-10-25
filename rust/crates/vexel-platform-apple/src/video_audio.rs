//! AAC audio track for `VideoWriter`: a 48 kHz stereo PCM16 WAV is decoded by
//! `AVAssetReader` and re-encoded by an `AVAssetWriterInput`. The reader only
//! hands over sample buffers; no Core Audio structures are built here.

use std::path::Path;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_av_foundation::{
    AVAssetReader, AVAssetReaderTrackOutput, AVAssetWriter, AVAssetWriterInput, AVMediaTypeAudio,
    AVURLAsset,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use crate::video_writer::VideoError;

/// `kAudioFormatLinearPCM` ('lpcm') and `kAudioFormatMPEG4AAC` ('aac ').
const FORMAT_LPCM: u32 = 0x6C70_636D;
const FORMAT_AAC: u32 = 0x6161_6320;
const SAMPLE_RATE: i32 = 48_000;
const CHANNELS: i32 = 2;
const AAC_BIT_RATE: i32 = 128_000;
const IO_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct AudioFeed {
    reader: Retained<AVAssetReader>,
    output: Retained<AVAssetReaderTrackOutput>,
    pub(crate) input: Retained<AVAssetWriterInput>,
    done: bool,
}

fn dict(pairs: &[(&str, Retained<NSNumber>)]) -> Retained<NSDictionary<NSString, AnyObject>> {
    let keys: Vec<Retained<NSString>> = pairs.iter().map(|(k, _)| NSString::from_str(k)).collect();
    let key_refs: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
    let value_refs: Vec<&AnyObject> = pairs.iter().map(|(_, v)| &***v as &AnyObject).collect();
    NSDictionary::from_slices(&key_refs, &value_refs)
}

fn native(what: &str) -> VideoError {
    VideoError::Native(format!("audio {what}"))
}

impl AudioFeed {
    /// Opens `wav` for reading and builds the (not yet added) AAC input.
    #[allow(deprecated)] // synchronous track load, as the test reader does
    pub(crate) fn open(wav: &Path) -> Result<Self, VideoError> {
        let path = wav
            .to_str()
            .ok_or_else(|| VideoError::Invalid("audio path is not UTF-8".into()))?;
        // SAFETY: retained objects used on this thread; statics are framework constants.
        unsafe {
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
            let media = AVMediaTypeAudio.ok_or_else(|| native("media type unavailable"))?;
            let tracks = asset.tracksWithMediaType(media);
            let track = tracks
                .firstObject()
                .ok_or_else(|| native("source has no track"))?;
            let reader = AVAssetReader::assetReaderWithAsset_error(&asset)
                .map_err(|e| VideoError::Native(e.localizedDescription().to_string()))?;
            let read_settings = dict(&[
                ("AVFormatIDKey", NSNumber::new_u32(FORMAT_LPCM)),
                ("AVLinearPCMBitDepthKey", NSNumber::new_i32(16)),
                ("AVLinearPCMIsBigEndianKey", NSNumber::new_bool(false)),
                ("AVLinearPCMIsFloatKey", NSNumber::new_bool(false)),
                ("AVLinearPCMIsNonInterleaved", NSNumber::new_bool(false)),
                ("AVSampleRateKey", NSNumber::new_i32(SAMPLE_RATE)),
                ("AVNumberOfChannelsKey", NSNumber::new_i32(CHANNELS)),
            ]);
            let output = AVAssetReaderTrackOutput::assetReaderTrackOutputWithTrack_outputSettings(
                &track,
                Some(&read_settings),
            );
            reader.addOutput(&output);
            if !reader.startReading() {
                return Err(native("reader failed to start"));
            }
            let write_settings = dict(&[
                ("AVFormatIDKey", NSNumber::new_u32(FORMAT_AAC)),
                ("AVSampleRateKey", NSNumber::new_i32(SAMPLE_RATE)),
                ("AVNumberOfChannelsKey", NSNumber::new_i32(CHANNELS)),
                ("AVEncoderBitRateKey", NSNumber::new_i32(AAC_BIT_RATE)),
            ]);
            let input = AVAssetWriterInput::assetWriterInputWithMediaType_outputSettings(
                media,
                Some(&write_settings),
            );
            input.setExpectsMediaDataInRealTime(false);
            Ok(Self {
                reader,
                output,
                input,
                done: false,
            })
        }
    }

    pub(crate) fn add_to(&self, writer: &AVAssetWriter) -> Result<(), VideoError> {
        // SAFETY: live writer and input.
        unsafe {
            if !writer.canAddInput(&self.input) {
                return Err(VideoError::Native("writer rejected audio input".into()));
            }
            writer.addInput(&self.input);
        }
        Ok(())
    }

    /// Appends buffers while the encoder accepts them; marks the input
    /// finished once the source is exhausted.
    pub(crate) fn pump(&mut self) -> Result<(), VideoError> {
        // SAFETY: the reader/output/input are live for the feed's lifetime.
        unsafe {
            while !self.done && self.input.isReadyForMoreMediaData() {
                match self.output.copyNextSampleBuffer() {
                    Some(sample) => {
                        if !self.input.appendSampleBuffer(&sample) {
                            return Err(native("append failed"));
                        }
                    }
                    None => {
                        if self.reader.status() == objc2_av_foundation::AVAssetReaderStatus::Failed
                        {
                            return Err(native("read failed"));
                        }
                        self.input.markAsFinished();
                        self.done = true;
                    }
                }
            }
        }
        Ok(())
    }

    /// Pumps until the whole source has been handed to the encoder.
    pub(crate) fn drain(&mut self, writer_failed: &dyn Fn() -> bool) -> Result<(), VideoError> {
        let start = Instant::now();
        while !self.done {
            self.pump()?;
            if self.done {
                break;
            }
            if writer_failed() {
                return Err(native("writer failed"));
            }
            if start.elapsed() > IO_TIMEOUT {
                return Err(VideoError::Timeout("waiting for audio encoder input"));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }
}
