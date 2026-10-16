//! CRC-32 (ISO 3309, as PNG chunks use) and Adler-32 (zlib trailer).

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

/// Continues a CRC-32 over `data`; `crc` is the value for the prefix.
pub(crate) fn crc32_update(crc: u32, data: &[u8]) -> u32 {
    let mut c = !crc;
    for &byte in data {
        c = CRC_TABLE[((c ^ u32::from(byte)) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

/// CRC-32 of `data` (`crc32(b"123456789") == 0xCBF43926`).
pub fn crc32(data: &[u8]) -> u32 {
    crc32_update(0, data)
}

/// Adler-32 of `data` (`adler32(b"Wikipedia") == 0x11E60398`).
pub fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    // The largest block for which the sums cannot overflow a u32.
    const NMAX: usize = 5552;
    let (mut a, mut b) = (1u32, 0u32);
    for block in data.chunks(NMAX) {
        for &byte in block {
            a += u32::from(byte);
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}
