//! Per-user location and source identity for native project recovery capsules.
//!
//! SHA-256 is implemented from the FIPS 180-4 specification (section 6.2.2),
//! with no external implementation or dependency.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StoreError {
    Unavailable,
    InvalidPath,
    OutOfMemory,
}

pub(super) fn capsule_path_for_canonical(canonical_bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
    let home = test_home_or_env()?;
    capsule_path_in_home(canonical_bytes, &home)
}

fn capsule_path_in_home(canonical_bytes: &[u8], home: &Path) -> Result<Vec<u8>, StoreError> {
    if canonical_bytes.is_empty() || canonical_bytes.contains(&0) || canonical_bytes.len() > 1023 {
        return Err(StoreError::InvalidPath);
    }
    let directory = secured_recovery_directory(home)?;
    let digest = sha256(canonical_bytes);
    let mut path = directory.into_os_string().into_encoded_bytes();
    let mut filename = Vec::new();
    filename
        .try_reserve_exact(64 + 5)
        .map_err(|_| StoreError::OutOfMemory)?;
    for byte in digest {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        filename.push(HEX[usize::from(byte >> 4)]);
        filename.push(HEX[usize::from(byte & 0x0f)]);
    }
    filename.extend_from_slice(b".vrec");
    path.try_reserve_exact(filename.len() + 1)
        .map_err(|_| StoreError::OutOfMemory)?;
    path.push(b'/');
    path.extend_from_slice(&filename);
    Ok(path)
}

fn test_home_or_env() -> Result<PathBuf, StoreError> {
    #[cfg(test)]
    if let Some(home) = TEST_HOME.with(|value| value.borrow().clone()) {
        return Ok(home);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(StoreError::Unavailable)
}

#[cfg(test)]
thread_local! {
    static TEST_HOME: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn with_test_home<R>(home: &Path, action: impl FnOnce() -> R) -> R {
    struct Reset(Option<PathBuf>);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_HOME.with(|value| *value.borrow_mut() = self.0.take());
        }
    }
    let previous = TEST_HOME.with(|value| value.borrow_mut().replace(home.to_path_buf()));
    let _reset = Reset(previous);
    action()
}

pub(super) fn canonical_source_path(source: &[u8]) -> Result<Vec<u8>, StoreError> {
    if source.is_empty() || source.contains(&0) {
        return Err(StoreError::InvalidPath);
    }
    let canonical = fs::canonicalize(Path::new(OsStr::from_bytes(source)))
        .map_err(|_| StoreError::InvalidPath)?;
    let canonical_bytes = canonical.as_os_str().as_bytes();
    if canonical_bytes.is_empty() || canonical_bytes.contains(&0) || canonical_bytes.len() > 1023 {
        return Err(StoreError::InvalidPath);
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(canonical_bytes.len())
        .map_err(|_| StoreError::OutOfMemory)?;
    result.extend_from_slice(canonical_bytes);
    Ok(result)
}

fn secured_recovery_directory(home: &Path) -> Result<PathBuf, StoreError> {
    let canonical_home = fs::canonicalize(home).map_err(|_| StoreError::Unavailable)?;
    let home_meta = fs::metadata(&canonical_home).map_err(|_| StoreError::Unavailable)?;
    if !home_meta.is_dir() {
        return Err(StoreError::Unavailable);
    }
    let mut directory = canonical_home;
    for component in ["Library", "Application Support", "Vexel"] {
        directory.push(component);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.file_type().is_dir() && metadata.uid() == home_meta.uid() => {}
            Ok(_) => return Err(StoreError::Unavailable),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                builder.mode(0o700);
                builder
                    .create(&directory)
                    .map_err(|_| StoreError::Unavailable)?;
            }
            Err(_) => return Err(StoreError::Unavailable),
        }
    }
    directory.push("Recovery");
    match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.file_type().is_dir() && metadata.uid() == home_meta.uid() => {
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| StoreError::Unavailable)?;
        }
        Ok(_) => return Err(StoreError::Unavailable),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder
                .create(&directory)
                .map_err(|_| StoreError::Unavailable)?;
        }
        Err(_) => return Err(StoreError::Unavailable),
    }
    let metadata = fs::symlink_metadata(&directory).map_err(|_| StoreError::Unavailable)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != home_meta.uid()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(StoreError::Unavailable);
    }
    Ok(directory)
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let blocks = input.len().saturating_add(9).saturating_add(63) / 64;
    for block_index in 0..blocks {
        let mut block = [0u8; 64];
        let start = block_index * 64;
        for (offset, slot) in block.iter_mut().enumerate() {
            let index = start + offset;
            if index < input.len() {
                *slot = input[index];
            } else if index == input.len() {
                *slot = 0x80;
            }
        }
        if block_index + 1 == blocks {
            block[56..].copy_from_slice(&bit_len.to_be_bytes());
        }
        let mut words = [0u32; 64];
        for (index, chunk) in block.chunks_exact(4).take(16).enumerate() {
            words[index] = u32::from_be_bytes(chunk.try_into().expect("four-byte chunk"));
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = (*slot).wrapping_add(value);
        }
    }
    let mut digest = [0u8; 32];
    for (chunk, word) in digest.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::sha256;

    #[test]
    fn sha256_matches_fips_known_answers() {
        assert_eq!(
            hex(sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    fn hex(bytes: [u8; 32]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
