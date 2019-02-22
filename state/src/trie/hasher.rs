extern crate hash256_std_hasher;
extern crate hash_db;
extern crate sha3;

use hash256_std_hasher::Hash256StdHasher;
use hash_db::Hasher;
use sha3::{Digest, Sha3_256};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Sha3Hasher;

impl Hasher for Sha3Hasher {
    type Out = [u8; 32];
    type StdHasher = Hash256StdHasher;

    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        let mut out = [0u8; Self::LENGTH];
        out.copy_from_slice(&Sha3_256::digest(x));
        out
    }
}

#[cfg(test)]
mod tests {
    extern crate hex;

    use super::*;

    #[test]
    fn test_sha3_hasher() {
        let out1 = Sha3Hasher::hash(b"test");
        let out2 = Sha3Hasher::hash(b"test");

        assert_eq!(out1, out2)
    }
}
