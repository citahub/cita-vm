//! Package hash_keccak is a function set for implementing Ethereum's state.
use super::err;
use cita_trie::codec::DataType;
use ethereum_types::H256;
use rlp::{Prototype, Rlp, RlpStream};
use tiny_keccak;

pub const NIL_DATA: H256 = H256([
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0, 0xe5, 0x00, 0xb6,
    0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
]);

pub const RLP_NULL: H256 = H256([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e, 0x5b, 0x48, 0xe0,
    0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

#[derive(Default, Debug)]
pub struct RLPNodeCodec {}

impl cita_trie::codec::NodeCodec for RLPNodeCodec {
    type Error = cita_trie::errors::RLPCodecError;

    const HASH_LENGTH: usize = 32;

    type Hash = [u8; 32];

    fn decode<F, T>(&self, data: &[u8], f: F) -> Result<T, Self::Error>
    where
        F: Fn(DataType) -> Result<T, Self::Error>,
    {
        let r = Rlp::new(data);
        match r.prototype()? {
            Prototype::Data(0) => Ok(f(DataType::Empty)?),
            Prototype::List(2) => {
                let key = r.at(0)?.data()?;
                let value = r.at(1)?.data()?;

                Ok(f(DataType::Pair(&key, &value))?)
            }
            Prototype::List(17) => {
                let mut values = vec![];
                for i in 0..17 {
                    values.push(r.at(i)?.as_raw().to_vec());
                }
                Ok(f(DataType::Values(&values))?)
            }
            _ => Ok(f(DataType::Hash(r.data()?))?),
        }
    }

    fn encode_empty(&self) -> Vec<u8> {
        let mut stream = RlpStream::new();
        stream.append_empty_data();
        stream.out()
    }

    fn encode_pair(&self, key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut stream = RlpStream::new_list(2);
        stream.append_raw(key, 1);
        stream.append_raw(value, 1);
        stream.out()
    }

    fn encode_values(&self, values: &[Vec<u8>]) -> Vec<u8> {
        let mut stream = RlpStream::new_list(values.len());
        for data in values {
            stream.append_raw(data, 1);
        }
        stream.out()
    }

    fn encode_raw(&self, raw: &[u8]) -> Vec<u8> {
        let mut stream = RlpStream::default();
        stream.append(&raw);
        stream.out()
    }

    fn decode_hash(&self, data: &[u8], is_hash: bool) -> Self::Hash {
        let mut out = [0u8; Self::HASH_LENGTH];
        if is_hash {
            out.copy_from_slice(data);
        } else {
            out.copy_from_slice(&tiny_keccak::keccak256(data)[..]);
        }
        out
    }
}

pub fn summary(data: &[u8]) -> Vec<u8> {
    tiny_keccak::keccak256(data).to_vec()
}

pub fn encodek(data: &[u8]) -> Vec<u8> {
    summary(data)
}

pub fn encodev<E: rlp::Encodable>(object: &E) -> Vec<u8> {
    rlp::encode(object)
}

pub fn decodev(data: &[u8]) -> Result<Vec<u8>, err::Error> {
    Ok(rlp::decode(&data)?)
}
