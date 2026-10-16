use vexel_project::{
    Asset, BundleInput, Error, MAX_ASSET_BYTES, MAX_ASSET_COUNT, MAX_ASSET_NAME_BYTES,
    MAX_ASSET_TOTAL_BYTES, MAX_MANIFEST_BYTES, MAX_MEDIA_TYPE_BYTES, MAX_SCENE_BYTES,
    MAX_TIMELINE_BYTES, decode, encode, encode_into, encoded_size, has_magic,
};

fn input<'a>(assets: &'a [Asset<'a>]) -> BundleInput<'a> {
    BundleInput { manifest_json: b"{}", scene_svg: b"<svg/>", timeline_json: b"{}", assets }
}

fn encoded(value: &BundleInput<'_>) -> Vec<u8> { encode(value).expect("valid bundle") }

fn error(bytes: &[u8], expected: Error) {
    assert_eq!(decode(bytes).unwrap_err().error, expected);
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn test_round_trip() {
    let image_bytes = [0, 1, 0xff, 0x20];
    let assets = [
        Asset { name: b"character.png", media_type: b"image/png", data: &image_bytes },
        Asset { name: b"audio-1.wav", media_type: b"audio/wav", data: b"" },
    ];
    let source = BundleInput {
        manifest_json: br#"{"title":"Sketch","version":1}"#,
        scene_svg: b"<svg xmlns='http://www.w3.org/2000/svg'><!--keep--></svg>",
        timeline_json: br#"{"fps":24,"tracks":[{"name":"walk"}]}"#,
        assets: &assets,
    };
    let measured = encoded_size(&source).unwrap();
    let bytes = encoded(&source);
    assert_eq!(bytes.len(), measured);
    assert!(has_magic(&bytes));
    let view = decode(&bytes).unwrap();
    assert_eq!(view.version, 1);
    assert_eq!(view.manifest_json, source.manifest_json);
    assert_eq!(view.scene_svg, source.scene_svg);
    assert_eq!(view.timeline_json, source.timeline_json);
    assert_eq!(view.asset_count, 2);
    for (index, expected) in assets.iter().enumerate() {
        assert_eq!(view.asset_at(index), Some(*expected));
    }
    assert_eq!(view.asset_at(2), None);
    let decoded_assets = [view.asset_at(0).unwrap(), view.asset_at(1).unwrap()];
    let roundtrip = BundleInput { manifest_json: view.manifest_json, scene_svg: view.scene_svg,
        timeline_json: view.timeline_json, assets: &decoded_assets };
    assert_eq!(encoded(&roundtrip), bytes);
}

#[test]
fn test_error_names() {
    assert_eq!(Error::Ok.name(), "ok");
    assert_eq!(Error::InvalidArgument.name(), "invalid_argument");
    assert_eq!(Error::Limit.name(), "limit");
    assert_eq!(Error::BufferTooSmall.name(), "buffer_too_small");
    assert_eq!(Error::BadMagic.name(), "bad_magic");
    assert_eq!(Error::UnsupportedVersion.name(), "unsupported_version");
    assert_eq!(Error::Corrupt.name(), "corrupt");
    assert_eq!(Error::OutOfMemory.name(), "out_of_memory");
}

#[test]
fn test_crc_vector_and_empty_timeline() {
    let empty = BundleInput { manifest_json: b"{}", scene_svg: b"<svg/>", timeline_json: b"", assets: &[] };
    assert_eq!(decode(&encoded(&empty)).unwrap().timeline_json, b"");
    let vector = BundleInput { manifest_json: b"123456789", ..empty };
    let bytes = encoded(&vector);
    assert_eq!(&bytes[40..44], &[0x26, 0x39, 0xf4, 0xcb]);
    assert!(decode(&bytes).is_ok());
}

#[test]
fn test_insufficient_and_invalid_inputs() {
    let no_assets = input(&[]);
    let size = encoded_size(&no_assets).unwrap();
    let mut output = vec![0xa5; size - 1];
    let failure = encode_into(&no_assets, &mut output).unwrap_err();
    assert_eq!(failure.error, Error::BufferTooSmall);
    assert_eq!(failure.encoded_size, size);
    assert!(output.iter().all(|byte| *byte == 0xa5));

    for bad in [
        Asset { name: b"../escape", media_type: b"image/png", data: b"x" },
        Asset { name: b"safe.png", media_type: b"image//png", data: b"x" },
    ] {
        assert_eq!(encoded_size(&input(std::slice::from_ref(&bad))).unwrap_err().error, Error::InvalidArgument);
    }
    let duplicate = [
        Asset { name: b"same", media_type: b"image/png", data: b"a" },
        Asset { name: b"same", media_type: b"image/jpeg", data: b"b" },
    ];
    assert_eq!(encoded_size(&input(&duplicate)).unwrap_err().error, Error::InvalidArgument);
    assert_eq!(encoded_size(&BundleInput { manifest_json: b"", ..no_assets }).unwrap_err().error, Error::Limit);
    assert_eq!(encoded_size(&BundleInput { scene_svg: b"", ..no_assets }).unwrap_err().error, Error::Limit);
    assert_eq!(encoded_size(&BundleInput { manifest_json: &[b'x'; MAX_MANIFEST_BYTES + 1], ..no_assets }).unwrap_err().error, Error::Limit);
    assert_eq!(encoded_size(&BundleInput { scene_svg: b"x", ..no_assets }).unwrap(), 61);
    let over_scene_limit = vec![0; MAX_SCENE_BYTES + 1];
    assert_eq!(encoded_size(&BundleInput { scene_svg: &over_scene_limit, ..no_assets }).unwrap_err().error, Error::Limit);
    assert_eq!(encoded_size(&BundleInput { timeline_json: &[b'x'; MAX_TIMELINE_BYTES + 1], ..no_assets }).unwrap_err().error, Error::Limit);
    let many = vec![Asset { name: b"a", media_type: b"x/y", data: b"" }; MAX_ASSET_COUNT + 1];
    assert_eq!(encoded_size(&input(&many)).unwrap_err().error, Error::Limit);
    let huge_asset = vec![0; MAX_ASSET_BYTES + 1];
    let oversized = Asset { name: b"large", media_type: b"image/png", data: &huge_asset };
    assert_eq!(encoded_size(&BundleInput { assets: &[oversized], ..no_assets }).unwrap_err().error, Error::Limit);

    // Safe Rust borrows prevent a mutable output slice from aliasing any input slice.
}

#[test]
fn test_exact_limits() {
    let manifest = vec![0; MAX_MANIFEST_BYTES];
    assert!(encoded_size(&BundleInput { manifest_json: &manifest, ..input(&[]) }).is_ok());
    let scene = vec![0; MAX_SCENE_BYTES];
    assert!(encoded_size(&BundleInput { scene_svg: &scene, ..input(&[]) }).is_ok());
    let timeline = vec![0; MAX_TIMELINE_BYTES];
    assert!(encoded_size(&BundleInput { timeline_json: &timeline, ..input(&[]) }).is_ok());
    let long_name = vec![b'n'; MAX_ASSET_NAME_BYTES];
    let mut long_type = vec![b't'; MAX_MEDIA_TYPE_BYTES];
    long_type[1] = b'/';
    let longest = [Asset { name: &long_name, media_type: &long_type, data: b"x" }];
    assert!(encoded_size(&input(&longest)).is_ok());

    let names: Vec<Vec<u8>> = (0..MAX_ASSET_COUNT).map(|i| format!("asset{i:04}").into_bytes()).collect();
    let many: Vec<_> = names.iter().map(|name| Asset { name, media_type: b"image/png", data: b"" }).collect();
    assert!(encoded_size(&input(&many)).is_ok());

    let first = vec![0; MAX_ASSET_BYTES];
    let second = vec![0; MAX_ASSET_BYTES];
    let max_pair = [
        Asset { name: b"large-a", media_type: b"application/octet-stream", data: &first },
        Asset { name: b"large-b", media_type: b"application/octet-stream", data: &second },
    ];
    assert_eq!(MAX_ASSET_TOTAL_BYTES, first.len() + second.len());
    assert!(encoded_size(&input(&max_pair)).is_ok());
    let over_total = [max_pair[0], max_pair[1], Asset { name: b"extra", media_type: b"image/png", data: b"x" }];
    assert_eq!(encoded_size(&input(&over_total)).unwrap_err().error, Error::Limit);
}

#[test]
fn test_corruption_and_headers() {
    let bytes = encoded(&input(&[]));
    let mut copy = bytes.clone();
    copy[0] ^= 1;
    error(&copy, Error::BadMagic);
    copy.clone_from(&bytes);
    copy[8] = 2;
    error(&copy, Error::UnsupportedVersion);
    copy.clone_from(&bytes);
    copy[12] = 55;
    error(&copy, Error::Corrupt);
    copy.clone_from(&bytes);
    copy[52] = 1;
    error(&copy, Error::Corrupt);
    for offset in [56, 58, 64] {
        copy.clone_from(&bytes);
        copy[offset] ^= 1;
        error(&copy, Error::Corrupt);
    }
    copy.clone_from(&bytes);
    copy[20] = 1;
    copy[21] = 4;
    assert_eq!(decode(&copy).unwrap_err().error, Error::Limit);
    copy.clone_from(&bytes);
    copy[24..32].fill(0xff);
    assert_eq!(decode(&copy).unwrap_err().error, Error::Limit);
    copy.clone_from(&bytes);
    copy[16..20].fill(0xff);
    assert_eq!(decode(&copy).unwrap_err().error, Error::Limit);
    error(&bytes[..bytes.len() - 1], Error::Corrupt);
    let mut trailing = bytes.clone();
    trailing.push(0);
    error(&trailing, Error::Corrupt);
}

#[test]
fn test_asset_validation() {
    let asset = Asset { name: b"sprite.bin", media_type: b"application/octet-stream", data: b"pixel" };
    let bytes = encoded(&input(&[asset]));
    let mut copy = bytes.clone();
    let last = copy.len() - 1;
    copy[last] ^= 1;
    let crc_failure = decode(&copy).unwrap_err();
    assert_eq!(crc_failure.error, Error::Corrupt);
    // project_bundle_decode reports its cursor after decode_asset_record has
    // consumed the asset header, name, media type, and data.
    assert_eq!(crc_failure.error_offset, 56 + 2 + 6 + 2 + 16 + 10 + 24 + 5);
    copy.clone_from(&bytes);
    let last = copy.len() - 1;
    copy[last - 20] = b'/';
    error(&copy, Error::Corrupt);
    copy.clone_from(&bytes);
    copy[64] = 0xff;
    error(&copy, Error::Corrupt);
    copy.clone_from(&bytes);
    copy[82] = b'S';
    error(&copy, Error::Corrupt);
    copy.clone_from(&bytes);
    copy[82 + asset.name.len()] = b'I';
    error(&copy, Error::Corrupt);
    copy.clone_from(&bytes);
    let data_length = asset.data.len();
    let crc_start = copy.len() - data_length - 8;
    copy[crc_start..crc_start + 8].fill(0xff);
    error(&copy, Error::Corrupt);

    let pair = [
        Asset { name: b"a.bin", media_type: b"image/png", data: b"a" },
        Asset { name: b"b.bin", media_type: b"image/png", data: b"b" },
    ];
    let mut duplicate = encoded(&input(&pair));
    let second_name = 56 + 2 + 6 + 2 + 16 + pair[0].name.len() + pair[0].media_type.len() + pair[0].data.len() + 16;
    duplicate[second_name] = b'a';
    // Recompute the second asset CRC so duplicate detection, not CRC validation, rejects it.
    let name_end = second_name + pair[1].name.len();
    let media_end = name_end + pair[1].media_type.len();
    let crc = crc32_ieee(&duplicate[second_name..media_end + pair[1].data.len()]);
    duplicate[second_name - 12..second_name - 8].copy_from_slice(&crc.to_le_bytes());
    error(&duplicate, Error::Corrupt);
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 { crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1)); }
    }
    !crc
}

#[test]
fn test_decoder_asset_limits() {
    let base = encoded(&input(&[]));
    let mut oversized = base.clone();
    put_u32(&mut oversized, 20, 1);
    oversized.extend_from_slice(&[1, 0, 3, 0, 0, 0, 0, 0]);
    oversized.extend_from_slice(&((MAX_ASSET_BYTES as u64) + 1).to_le_bytes());
    assert_eq!(decode(&oversized).unwrap_err().error, Error::Limit);

    let mut excessive_total = base;
    put_u32(&mut excessive_total, 20, 3);
    for name in [b'a', b'b', b'c'] {
        let data_len = if name == b'c' { 1usize } else { MAX_ASSET_BYTES };
        let mut header = [0; 16];
        header[0..2].copy_from_slice(&1u16.to_le_bytes());
        header[2..4].copy_from_slice(&3u16.to_le_bytes());
        header[8..16].copy_from_slice(&(data_len as u64).to_le_bytes());
        excessive_total.extend_from_slice(&header);
        excessive_total.push(name);
        excessive_total.extend_from_slice(b"x/y");
        excessive_total.resize(excessive_total.len() + data_len, 0);
    }
    assert_eq!(decode(&excessive_total).unwrap_err().error, Error::Limit);
}

