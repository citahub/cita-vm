use ethereum_types::H256;
use sha3::{Digest, Sha3_256};

pub trait Hasher: Clone {
    fn digest(data: &[u8]) -> H256;
}

#[derive(Clone)]
pub struct Sha3Hasher {}

impl Sha3Hasher {
    pub fn new() -> Self {
        Sha3Hasher {}
    }
}

impl Hasher for Sha3Hasher {
    fn digest(data: &[u8]) -> H256 {
        let result = Sha3_256::digest(data);
        H256::from_slice(result.as_slice())
    }
}
