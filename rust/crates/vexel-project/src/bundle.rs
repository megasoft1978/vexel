use std::collections::TryReserveError;

pub const VERSION: u32 = 1;
pub const HEADER_BYTES: usize = 56;
pub const MAX_BYTES: usize = 1_073_741_824;
pub const MAX_MANIFEST_BYTES: usize = 1_048_576;
pub const MAX_SCENE_BYTES: usize = 268_435_456;
pub const MAX_TIMELINE_BYTES: usize = 67_108_864;
pub const MAX_ASSET_COUNT: usize = 1024;
pub const MAX_ASSET_NAME_BYTES: usize = 128;
pub const MAX_MEDIA_TYPE_BYTES: usize = 128;
pub const MAX_ASSET_BYTES: usize = 268_435_456;
pub const MAX_ASSET_TOTAL_BYTES: usize = 536_870_912;
pub const ASSET_HEADER_BYTES: usize = 16;
pub const MAGIC: &[u8; 8] = b"VEXEL\r\n\x1a";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    Ok,
    InvalidArgument,
    Limit,
    BufferTooSmall,
    BadMagic,
    UnsupportedVersion,
    Corrupt,
    OutOfMemory,
}

impl Error {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::InvalidArgument => "invalid_argument",
            Self::Limit => "limit",
            Self::BufferTooSmall => "buffer_too_small",
            Self::BadMagic => "bad_magic",
            Self::UnsupportedVersion => "unsupported_version",
            Self::Corrupt => "corrupt",
            Self::OutOfMemory => "out_of_memory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleFailure {
    pub error: Error,
    pub encoded_size: usize,
    pub error_offset: usize,
}

impl BundleFailure {
    const fn new(error: Error, encoded_size: usize, error_offset: usize) -> Self {
        Self { error, encoded_size, error_offset }
    }
}

pub type Result<T> = std::result::Result<T, BundleFailure>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Asset<'a> {
    pub name: &'a [u8],
    pub media_type: &'a [u8],
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleInput<'a> {
    pub manifest_json: &'a [u8],
    pub scene_svg: &'a [u8],
    pub timeline_json: &'a [u8],
    pub assets: &'a [Asset<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleView<'a> {
    pub version: u32,
    pub manifest_json: &'a [u8],
    pub scene_svg: &'a [u8],
    pub timeline_json: &'a [u8],
    pub encoded: &'a [u8],
    pub asset_count: usize,
    asset_start: usize,
}

impl<'a> BundleView<'a> {
    /// Returns an asset whose byte slices remain tied to the encoded input.
    pub fn asset_at(&self, index: usize) -> Option<Asset<'a>> {
        if index >= self.asset_count { return None; }
        let mut offset = self.asset_start;
        for _ in 0..index {
            let header = self.encoded.get(offset..offset.checked_add(ASSET_HEADER_BYTES)?)?;
            let name_len = usize::from(read_u16(header, 0)?);
            let media_len = usize::from(read_u16(header, 2)?);
            let data_len = usize::try_from(read_u64(header, 8)?).ok()?;
            offset = offset.checked_add(ASSET_HEADER_BYTES)?
                .checked_add(name_len)?.checked_add(media_len)?.checked_add(data_len)?;
        }
        read_asset(self.encoded, &mut offset).ok()
    }
}

fn failure(error: Error) -> BundleFailure { BundleFailure::new(error, 0, 0) }

fn checked_add(total: usize, amount: usize) -> Option<usize> { total.checked_add(amount) }

fn valid_name(name: &[u8]) -> bool {
    !name.is_empty() && name.len() <= MAX_ASSET_NAME_BYTES && name != b"." && name != b".."
        && name.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-'))
}

fn valid_media_type(media: &[u8]) -> bool {
    if !(3..=MAX_MEDIA_TYPE_BYTES).contains(&media.len()) { return false; }
    let mut slash = None;
    for (index, byte) in media.iter().copied().enumerate() {
        if byte == b'/' {
            if slash.is_some() || index == 0 || index + 1 == media.len() { return false; }
            slash = Some(index);
        } else if !(0x21..=0x7e).contains(&byte) { return false; }
    }
    slash.is_some()
}

fn crc32_update(mut crc: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    crc
}

fn crc32(bytes: &[u8]) -> u32 { !crc32_update(u32::MAX, bytes) }

fn asset_crc32(asset: Asset<'_>) -> u32 {
    let crc = crc32_update(u32::MAX, asset.name);
    let crc = crc32_update(crc, asset.media_type);
    !crc32_update(crc, asset.data)
}

fn validate_input(input: &BundleInput<'_>) -> Result<usize> {
    if input.manifest_json.is_empty() || input.scene_svg.is_empty()
        || input.manifest_json.len() > MAX_MANIFEST_BYTES
        || input.scene_svg.len() > MAX_SCENE_BYTES
        || input.timeline_json.len() > MAX_TIMELINE_BYTES
        || input.assets.len() > MAX_ASSET_COUNT
    { return Err(failure(Error::Limit)); }

    let mut total = HEADER_BYTES;
    for length in [input.manifest_json.len(), input.scene_svg.len(), input.timeline_json.len()] {
        total = checked_add(total, length).ok_or_else(|| failure(Error::Limit))?;
    }
    let mut asset_total = 0usize;
    for (index, asset) in input.assets.iter().enumerate() {
        if !valid_name(asset.name) || !valid_media_type(asset.media_type) {
            return Err(failure(Error::InvalidArgument));
        }
        if asset.data.len() > MAX_ASSET_BYTES {
            return Err(failure(Error::Limit));
        }
        asset_total = checked_add(asset_total, asset.data.len()).ok_or_else(|| failure(Error::Limit))?;
        if asset_total > MAX_ASSET_TOTAL_BYTES { return Err(failure(Error::Limit)); }
        if input.assets[..index].iter().any(|prior| prior.name == asset.name) {
            return Err(failure(Error::InvalidArgument));
        }
        let record = ASSET_HEADER_BYTES.checked_add(asset.name.len())
            .and_then(|n| n.checked_add(asset.media_type.len()))
            .and_then(|n| n.checked_add(asset.data.len()))
            .ok_or_else(|| failure(Error::Limit))?;
        total = checked_add(total, record).ok_or_else(|| failure(Error::Limit))?;
    }
    if total > MAX_BYTES { return Err(failure(Error::Limit)); }
    Ok(total)
}

/// Computes the exact encoded length after applying all v1 input constraints.
pub fn encoded_size(input: &BundleInput<'_>) -> Result<usize> { validate_input(input) }

/// Encodes into caller storage. Validation completes before any bytes are written.
pub fn encode_into(input: &BundleInput<'_>, output: &mut [u8]) -> Result<usize> {
    let size = validate_input(input)?;
    if output.len() < size {
        return Err(BundleFailure::new(Error::BufferTooSmall, size, 0));
    }
    output[..8].copy_from_slice(MAGIC);
    put_u32(output, 8, VERSION);
    put_u32(output, 12, HEADER_BYTES as u32);
    put_u32(output, 16, input.manifest_json.len() as u32);
    put_u32(output, 20, input.assets.len() as u32);
    put_u64(output, 24, input.scene_svg.len() as u64);
    put_u64(output, 32, input.timeline_json.len() as u64);
    put_u32(output, 40, crc32(input.manifest_json));
    put_u32(output, 44, crc32(input.scene_svg));
    put_u32(output, 48, crc32(input.timeline_json));
    put_u32(output, 52, 0);

    let mut offset = HEADER_BYTES;
    for bytes in [input.manifest_json, input.scene_svg, input.timeline_json] {
        output[offset..offset + bytes.len()].copy_from_slice(bytes);
        offset += bytes.len();
    }
    for asset in input.assets {
        put_u16(output, offset, asset.name.len() as u16);
        put_u16(output, offset + 2, asset.media_type.len() as u16);
        put_u32(output, offset + 4, asset_crc32(*asset));
        put_u64(output, offset + 8, asset.data.len() as u64);
        offset += ASSET_HEADER_BYTES;
        for bytes in [asset.name, asset.media_type, asset.data] {
            output[offset..offset + bytes.len()].copy_from_slice(bytes);
            offset += bytes.len();
        }
    }
    Ok(offset)
}

/// Produces an owned bundle, reporting allocator failure instead of panicking.
pub fn encode(input: &BundleInput<'_>) -> Result<Vec<u8>> {
    let size = validate_input(input)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_: TryReserveError| failure(Error::OutOfMemory))?;
    bytes.resize(size, 0);
    encode_into(input, &mut bytes)?;
    Ok(bytes)
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?))
}
fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?))
}
fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?))
}
fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_asset<'a>(encoded: &'a [u8], offset: &mut usize) -> Result<Asset<'a>> {
    let header_end = offset.checked_add(ASSET_HEADER_BYTES).ok_or_else(|| failure(Error::Corrupt))?;
    let header = encoded.get(*offset..header_end).ok_or_else(|| failure(Error::Corrupt))?;
    let name_len = usize::from(read_u16(header, 0).ok_or_else(|| failure(Error::Corrupt))?);
    let media_len = usize::from(read_u16(header, 2).ok_or_else(|| failure(Error::Corrupt))?);
    let expected_crc = read_u32(header, 4).ok_or_else(|| failure(Error::Corrupt))?;
    let data_len64 = read_u64(header, 8).ok_or_else(|| failure(Error::Corrupt))?;
    if data_len64 > usize::MAX as u64 {
        return Err(failure(Error::Limit));
    }
    let data_len = data_len64 as usize;
    *offset = header_end;
    let name_end = offset.checked_add(name_len).ok_or_else(|| failure(Error::Corrupt))?;
    let name = encoded.get(*offset..name_end).ok_or_else(|| failure(Error::Corrupt))?;
    *offset = name_end;
    let media_end = offset.checked_add(media_len).ok_or_else(|| failure(Error::Corrupt))?;
    let media_type = encoded.get(*offset..media_end).ok_or_else(|| failure(Error::Corrupt))?;
    *offset = media_end;
    let end = offset.checked_add(data_len).ok_or_else(|| failure(Error::Corrupt))?;
    let data = encoded.get(*offset..end).ok_or_else(|| failure(Error::Corrupt))?;
    *offset = end;
    let asset = Asset { name, media_type, data };
    if !valid_name(name) || !valid_media_type(media_type)
        || data_len > MAX_ASSET_BYTES || asset_crc32(asset) != expected_crc
    { return Err(failure(Error::Corrupt)); }
    Ok(asset)
}

fn preflight_asset_records(encoded: &[u8], mut offset: usize, count: usize) -> Result<()> {
    let mut total_data = 0usize;
    for _ in 0..count {
        let header_end = offset.checked_add(ASSET_HEADER_BYTES)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?;
        let header = encoded.get(offset..header_end)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?;
        let name_len = usize::from(read_u16(header, 0).ok_or_else(|| failure(Error::Corrupt))?);
        let media_len = usize::from(read_u16(header, 2).ok_or_else(|| failure(Error::Corrupt))?);
        let data_len64 = read_u64(header, 8).ok_or_else(|| failure(Error::Corrupt))?;
        if data_len64 > usize::MAX as u64 || data_len64 > MAX_ASSET_BYTES as u64 {
            return Err(BundleFailure::new(Error::Limit, 0, offset + 8));
        }
        total_data = total_data.checked_add(data_len64 as usize)
            .ok_or_else(|| BundleFailure::new(Error::Limit, 0, offset + 8))?;
        if total_data > MAX_ASSET_TOTAL_BYTES {
            return Err(BundleFailure::new(Error::Limit, 0, offset + 8));
        }
        let record_len = ASSET_HEADER_BYTES.checked_add(name_len)
            .and_then(|n| n.checked_add(media_len))
            .and_then(|n| n.checked_add(data_len64 as usize))
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?;
        let end = offset.checked_add(record_len)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?;
        let name = encoded.get(header_end..header_end.checked_add(name_len)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?;
        let media_start = header_end + name_len;
        let media = encoded.get(media_start..media_start.checked_add(media_len)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?)
            .ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, offset))?;
        if end > encoded.len() { return Err(BundleFailure::new(Error::Corrupt, 0, offset)); }
        if !valid_name(name) || !valid_media_type(media) {
            return Err(BundleFailure::new(Error::Corrupt, 0, header_end));
        }
        offset = end;
    }
    if offset != encoded.len() { return Err(BundleFailure::new(Error::Corrupt, 0, offset)); }
    Ok(())
}

/// Validates a complete v1 bundle and returns a view borrowing its input bytes.
pub fn decode(encoded: &[u8]) -> Result<BundleView<'_>> {
    if encoded.len() < HEADER_BYTES || encoded.len() > MAX_BYTES {
        return Err(BundleFailure::new(Error::Corrupt, 0, encoded.len()));
    }
    if !has_magic(encoded) { return Err(failure(Error::BadMagic)); }
    let version = read_u32(encoded, 8).ok_or_else(|| failure(Error::Corrupt))?;
    if version != VERSION { return Err(BundleFailure::new(Error::UnsupportedVersion, 0, 8)); }
    if read_u32(encoded, 12) != Some(HEADER_BYTES as u32) || read_u32(encoded, 52) != Some(0) {
        return Err(BundleFailure::new(Error::Corrupt, 0, 12));
    }
    let manifest_len = read_u32(encoded, 16).ok_or_else(|| failure(Error::Corrupt))? as usize;
    let asset_count = read_u32(encoded, 20).ok_or_else(|| failure(Error::Corrupt))? as usize;
    let scene_len64 = read_u64(encoded, 24).ok_or_else(|| failure(Error::Corrupt))?;
    let timeline_len64 = read_u64(encoded, 32).ok_or_else(|| failure(Error::Corrupt))?;
    if manifest_len > MAX_MANIFEST_BYTES || scene_len64 == 0 || scene_len64 > MAX_SCENE_BYTES as u64
        || timeline_len64 > MAX_TIMELINE_BYTES as u64 || asset_count > MAX_ASSET_COUNT
    { return Err(BundleFailure::new(Error::Limit, 0, 16)); }
    let scene_len = scene_len64 as usize;
    let timeline_len = timeline_len64 as usize;
    let manifest_start = HEADER_BYTES;
    let scene_start = manifest_start.checked_add(manifest_len).ok_or_else(|| failure(Error::Corrupt))?;
    let timeline_start = scene_start.checked_add(scene_len).ok_or_else(|| failure(Error::Corrupt))?;
    let asset_start = timeline_start.checked_add(timeline_len).ok_or_else(|| failure(Error::Corrupt))?;
    let manifest_json = encoded.get(manifest_start..scene_start).ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, scene_start))?;
    let scene_svg = encoded.get(scene_start..timeline_start).ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, timeline_start))?;
    let timeline_json = encoded.get(timeline_start..asset_start).ok_or_else(|| BundleFailure::new(Error::Corrupt, 0, asset_start))?;
    if manifest_len == 0 || crc32(manifest_json) != read_u32(encoded, 40).unwrap_or_default()
        || crc32(scene_svg) != read_u32(encoded, 44).unwrap_or_default()
        || crc32(timeline_json) != read_u32(encoded, 48).unwrap_or_default()
    { return Err(BundleFailure::new(Error::Corrupt, 0, HEADER_BYTES)); }

    preflight_asset_records(encoded, asset_start, asset_count)?;
    let mut offset = asset_start;
    let mut names: Vec<&[u8]> = Vec::new();
    names.try_reserve_exact(asset_count).map_err(|_| failure(Error::OutOfMemory))?;
    for _ in 0..asset_count {
        let asset = read_asset(encoded, &mut offset).map_err(|_| {
            BundleFailure::new(Error::Corrupt, 0, offset)
        })?;
        if names.iter().any(|name| *name == asset.name) {
            return Err(BundleFailure::new(Error::Corrupt, 0, offset));
        }
        names.push(asset.name);
    }
    if offset != encoded.len() { return Err(BundleFailure::new(Error::Corrupt, 0, offset)); }
    Ok(BundleView { version, manifest_json, scene_svg, timeline_json, encoded, asset_count, asset_start })
}

/// Checks the fixed eight-byte bundle signature.
pub fn has_magic(encoded: &[u8]) -> bool { encoded.starts_with(MAGIC) }

