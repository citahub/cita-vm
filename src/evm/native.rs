//! List of precompiled contracts:
//!
//!   1. Recovery of ECDSA signature
//!   2. Hash function SHA256
//!   3. Hash function RIPEMD160
//!   4. Identity
//! Since Byzantium fork
//!   5. Modular exponentiation
//!   6. Addition on elliptic curve alt_bn128
//!   7. Scalar multiplication on elliptic curve alt_bn128
//!   8. Checking a pairing equation on curve alt_bn128
use std::io::Write;

use ethereum_types::{Address, H256, H512, U256};
use ripemd160::{Digest, Ripemd160};
use sha2::Sha256;

use crate::common;
use crate::evm::err::Error;

/// Implementation of a pre-compiled contract.
pub trait PrecompiledContract: Send + Sync {
    /// Return required gas for contract call.
    fn required_gas(&self, input: &[u8]) -> u64;

    /// Get the output from the pre-compiled contract.
    fn run(&self, input: &[u8]) -> Result<Vec<u8>, Error>;
}

/// Function get returns a pre-compiled contract by given address.
pub fn get(address: Address) -> Box<dyn PrecompiledContract> {
    match U256::from(H256::from(address)).low_u64() {
        0x01 => Box::new(EcRecover {}) as Box<dyn PrecompiledContract>,
        0x02 => Box::new(SHA256Hash {}) as Box<dyn PrecompiledContract>,
        0x03 => Box::new(RIPEMD160Hash {}) as Box<dyn PrecompiledContract>,
        0x04 => Box::new(DataCopy {}) as Box<dyn PrecompiledContract>,
        0x05 => Box::new(BigModExp {}) as Box<dyn PrecompiledContract>,
        0x06 => Box::new(Bn256Add {}) as Box<dyn PrecompiledContract>,
        0x07 => Box::new(Bn256ScalarMul {}) as Box<dyn PrecompiledContract>,
        0x08 => Box::new(Bn256Pairing {}) as Box<dyn PrecompiledContract>,
        _ => unimplemented!(),
    }
}

/// Check if an address is pre-compiled contract.
pub fn contains(address: &Address) -> bool {
    let i = U256::from(H256::from(address));
    i <= U256::from(8) && !i.is_zero()
}

const G_ECRECOVER: u64 = 3000; // Elliptic curve sender recovery gas price
const G_SHA256_BASE: u64 = 60; // Base price for a SHA256 operation
const G_SHA256_PER_WORD: u64 = 12; // Per-word price for a SHA256 operation
const G_RIPEMD160_BASE: u64 = 600; // Base price for a RIPEMD160 operation
const G_RIPEMD160_PER_WORD: u64 = 120; // Per-word price for a RIPEMD160 operation
const G_IDENTITY_BASE: u64 = 15; // Base price for a data copy operation
const G_IDENTITY_PER_WORD: u64 = 3; // Per-work price for a data copy operation
                                    // const G_MOD_EXP_QUADCOEFF_DIV: u64 = 20; // Divisor for the quadratic particle of the big int modular exponentiation
const G_BN256_ADD: u64 = 500; // Gas needed for an elliptic curve addition
const G_BN256_SCALAR_MUL: u64 = 40000; // Gas needed for an elliptic curve scalar multiplication
const G_BN256_PARING_BASE: u64 = 100_000; // Base price for an elliptic curve pairing check
const G_BN256_PARING_PER_POINT: u64 = 80000; // Per-point price for an elliptic curve pairing check

/// Check if each component of the signature is in range.
fn is_signature_valid(r: &H256, s: &H256, v: u8) -> bool {
    v <= 1
        && *r < H256::from("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
        && *r >= H256::from(1)
        && *s < H256::from("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
        && *s >= H256::from(1)
}

/// Recover public from signed messages.
fn recover(input: &[u8], hash: &[u8], bit: u8) -> Result<H512, secp256k1::Error> {
    let signature = secp256k1::Signature::parse_slice(&input[64..128])?;
    let message = secp256k1::Message::parse_slice(&hash[..])?;
    let recovery_id = secp256k1::RecoveryId::parse(bit)?;
    let pub_key = secp256k1::recover(&message, &signature, &recovery_id)?;
    let pub_key_ser = pub_key.serialize();
    Ok(H512::from(&pub_key_ser[1..65]))
}

/// ECRECOVER implemented as a native contract.
pub struct EcRecover {}

impl PrecompiledContract for EcRecover {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, Error> {
        let len = std::cmp::min(i.len(), 128);

        let mut input = [0; 128];
        input[..len].copy_from_slice(&i[..len]);

        let hash = H256::from(&input[0..32]);
        let v = H256::from(&input[32..64]);
        let r = H256::from(&input[64..96]);
        let s = H256::from(&input[96..128]);

        let bit = match v[31] {
            27 | 28 if v.0[..31] == [0; 31] => v[31] - 27,
            _ => {
                return Ok(vec![]);
            }
        };
        if !is_signature_valid(&r, &s, bit) {
            return Ok(vec![]);
        }
        let mut output: Vec<u8> = Vec::new();
        if let Ok(public) = recover(&input, &hash, bit) {
            let data = common::hash::summary(&public.0);
            output.write_all(&[0; 12]).unwrap();
            output.write_all(&data[12..data.len()]).unwrap();
        }
        Ok(output)
    }
}

/// SHA256 implemented as a native contract.
pub struct SHA256Hash {}

impl PrecompiledContract for SHA256Hash {
    // This method does not require any overflow checking as the input size gas costs
    // required for anything significant is so high it's impossible to pay for.
    fn required_gas(&self, i: &[u8]) -> u64 {
        (i.len() as u64 + 31) / 32 * G_SHA256_PER_WORD + G_SHA256_BASE
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, Error> {
        let mut hasher = Sha256::new();
        hasher.input(i);
        let result = hasher.result();
        let mut output: Vec<u8> = Vec::new();
        output.write_all(&result).unwrap();
        Ok(output)
    }
}

/// RIPEMD160 implemented as a native contract.
pub struct RIPEMD160Hash {}

impl PrecompiledContract for RIPEMD160Hash {
    // This method does not require any overflow checking as the input size gas costs
    // required for anything significant is so high it's impossible to pay for.
    fn required_gas(&self, i: &[u8]) -> u64 {
        (i.len() as u64 + 31) / 32 * G_RIPEMD160_PER_WORD + G_RIPEMD160_BASE
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, Error> {
        let mut hasher = Ripemd160::new();
        hasher.input(i);
        let result = hasher.result();
        let mut output: Vec<u8> = Vec::new();
        output.write_all(&[0; 12]).unwrap();
        output.write_all(&result).unwrap();
        Ok(output)
    }
}

pub struct DataCopy {}

impl PrecompiledContract for DataCopy {
    // This method does not require any overflow checking as the input size gas costs
    // required for anything significant is so high it's impossible to pay for.
    fn required_gas(&self, i: &[u8]) -> u64 {
        (i.len() as u64 + 31) / 32 * G_IDENTITY_PER_WORD + G_IDENTITY_BASE
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(i.into())
    }
}

/// BigModExp implements a native big integer exponential modular operation.
pub struct BigModExp {}

impl PrecompiledContract for BigModExp {
    fn required_gas(&self, _: &[u8]) -> u64 {
        0
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, Error> {
        Err(Error::CallError)
    }
}

/// Bn256Add implements a native elliptic curve point addition.
pub struct Bn256Add {}

impl PrecompiledContract for Bn256Add {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_BN256_ADD
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, Error> {
        Err(Error::CallError)
    }
}

/// Bn256ScalarMul implements a native elliptic curve point addition.
pub struct Bn256ScalarMul {}

impl PrecompiledContract for Bn256ScalarMul {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_BN256_SCALAR_MUL
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, Error> {
        Err(Error::CallError)
    }
}

/// Bn256Pairing implements a pairing pre-compile for the bn256 curve
pub struct Bn256Pairing {}

impl PrecompiledContract for Bn256Pairing {
    fn required_gas(&self, i: &[u8]) -> u64 {
        G_BN256_PARING_BASE + (i.len() as u64 / 192 * G_BN256_PARING_PER_POINT)
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, Error> {
        Err(Error::CallError)
    }
}
