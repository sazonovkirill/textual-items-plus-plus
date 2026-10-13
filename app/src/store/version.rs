//! The fingerprint that tells whether a file on disk still holds what the app last read or wrote
//! there ([file-io.md §3](../../../specs/reference/taskpaper-terminal/specs/file-io.md#3-external-changes)).

/// A fingerprint of a file's bytes: their length and hash. It is only compared within one
/// session, so a collision can at worst take a changed file for an unchanged one of equal length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    len: usize,
    hash: u64,
}

impl Version {
    /// The version of a file holding `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            len: bytes.len(),
            hash: fnv1a(bytes),
        }
    }
}

/// The 64-bit FNV-1a hash, written out so that names made from it stay the same across Rust
/// releases, which std's `DefaultHasher` does not promise.
pub(super) fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_with_fnv1a() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn equal_bytes_have_one_version_and_different_bytes_another() {
        assert_eq!(Version::of(b"- a\n"), Version::of(b"- a\n"));
        assert_eq!(Version::of(b""), Version::of(&[]));
        assert_ne!(Version::of(b"- a\n"), Version::of(b"- b\n"));
        assert_ne!(Version::of(b"- a\n"), Version::of(b"- a\n\n"));
        assert_ne!(Version::of("é".as_bytes()), Version::of(b"e"));
    }
}
