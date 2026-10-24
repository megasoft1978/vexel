// wincap: record ONLY one app window (ScreenCaptureKit desktop-independent
// window filter, matched by owning pid) to H.264 at the window's native pixel size.
// usage: wincap --pid P --out file.mp4 [--fps 60] [--seconds N]
// Stops at --seconds, SIGINT/SIGTERM, 'q' on stdin, or when the stream errors
// (window closed): the file is finalized in all cases.
// Build: swiftc -O wincap.swift -o wincap   (needs Screen Recording permission)
import Foundation
import AVFoundation
import ScreenCaptureKit
import CoreMedia
import AppKit

var outPath = ""
var seconds: Double = 0
var pid: Int32 = 0
var fps = 60
var args = Array(CommandLine.arguments.dropFirst())
while !args.isEmpty {
    let a = args.removeFirst()
    switch a {
    case "--out": outPath = args.removeFirst()
    case "--seconds": seconds = Double(args.removeFirst()) ?? 0
    case "--pid": pid = Int32(args.removeFirst()) ?? 0
    case "--fps": fps = Int(args.removeFirst()) ?? 60
    default: FileHandle.standardError.write("unknown arg \(a)\n".data(using: .utf8)!); exit(2)
    }
}
func die(_ s: String) -> Never { FileHandle.standardError.write((s + "\n").data(using: .utf8)!); exit(1) }
if outPath.isEmpty || pid == 0 { die("need --pid and --out") }

final class Recorder: NSObject, SCStreamOutput, SCStreamDelegate {
    var writer: AVAssetWriter!
    var input: AVAssetWriterInput!
    let url: URL
    let w: Int, h: Int
    var started = false
    var finished = false
    var frames = 0
    var onStop: (() -> Void)?
    init(url: URL, w: Int, h: Int) {
        self.url = url; self.w = w; self.h = h
        try? FileManager.default.removeItem(at: url)
        super.init()
        writer = try! AVAssetWriter(outputURL: url, fileType: .mp4)
        input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: w, AVVideoHeightKey: h,
            AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 30_000_000,
                                              AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel]])
        input.expectsMediaDataInRealTime = true
        writer.add(input)
    }
    func stream(_ s: SCStream, didOutputSampleBuffer sb: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .screen, sb.isValid, !finished,
              let arr = CMSampleBufferGetSampleAttachmentsArray(sb, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let raw = arr.first?[.status] as? Int, SCFrameStatus(rawValue: raw) == .complete else { return }
        if !started {
            writer.startWriting()
            writer.startSession(atSourceTime: CMSampleBufferGetPresentationTimeStamp(sb))
            started = true
        }
        if input.isReadyForMoreMediaData { input.append(sb); frames += 1 }
    }
    func stream(_ s: SCStream, didStopWithError e: Error) {
        FileHandle.standardError.write("stream error: \(e)\n".data(using: .utf8)!)
        onStop?()
    }
    func finalize(_ done: @escaping () -> Void) {
        if finished { return }; finished = true
        guard started else { done(); return }
        input.markAsFinished()
        writer.finishWriting { done() }
    }
}

var keep: [Any] = []
_ = NSApplication.shared  // initialise CoreGraphics session for ScreenCaptureKit in a CLI
Task {
    do {
        let content = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
        let wins = content.windows.filter {
            $0.owningApplication?.processID == pid && $0.frame.width > 200 && $0.frame.height > 200
        }.sorted { $0.frame.width * $0.frame.height > $1.frame.width * $1.frame.height }
        guard let win = wins.first else { die("no on-screen window for pid \(pid) (Screen Recording permission?)") }
        let filter = SCContentFilter(desktopIndependentWindow: win)
        let scale = Int(filter.pointPixelScale)
        let W = (Int(win.frame.width) * scale) & ~1, H = (Int(win.frame.height) * scale) & ~1
        print("window id=\(win.windowID) frame=\(win.frame) px=\(W)x\(H)")
        let cfg = SCStreamConfiguration()
        cfg.width = W; cfg.height = H
        cfg.minimumFrameInterval = CMTime(value: 1, timescale: CMTimeScale(fps))
        cfg.pixelFormat = kCVPixelFormatType_32BGRA
        cfg.showsCursor = true
        cfg.queueDepth = 8
        let rec = Recorder(url: URL(fileURLWithPath: outPath), w: W, h: H)
        let stream = SCStream(filter: filter, configuration: cfg, delegate: rec)
        try stream.addStreamOutput(rec, type: .screen, sampleHandlerQueue: DispatchQueue(label: "cap"))
        try await stream.startCapture()
        print("recording"); fflush(stdout)
        var stopping = false
        let finish: () -> Void = {
            if stopping { return }; stopping = true
            Task {
                try? await stream.stopCapture()
                rec.finalize {
                    print("done frames=\(rec.frames) out=\(outPath)"); fflush(stdout)
                    exit(rec.frames > 0 ? 0 : 1)
                }
            }
        }
        rec.onStop = { stopping = true; rec.finalize { print("done(after error) frames=\(rec.frames)"); exit(rec.frames > 0 ? 0 : 1) } }
        for sig in [SIGINT, SIGTERM] {
            signal(sig, SIG_IGN)
            let src = DispatchSource.makeSignalSource(signal: sig, queue: .main)
            src.setEventHandler(handler: finish); src.resume(); keep.append(src)
        }
        let stdinSrc = DispatchSource.makeReadSource(fileDescriptor: 0, queue: .main)
        stdinSrc.setEventHandler {
            var buf = [UInt8](repeating: 0, count: 64)
            let n = read(0, &buf, 64)
            if n <= 0 { stdinSrc.cancel(); return }
            if buf[0..<n].contains(UInt8(ascii: "q")) { finish() }
        }
        stdinSrc.resume(); keep.append(stdinSrc)
        if seconds > 0 { DispatchQueue.main.asyncAfter(deadline: .now() + seconds, execute: finish) }
    } catch { die("capture failed: \(error)") }
}
RunLoop.main.run()
