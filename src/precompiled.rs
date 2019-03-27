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
use super::err;
use ethereum_types::{Address, H256, H512, U256};
use num::Zero;
use numext_fixed_uint::{U256 as EU256, U4096 as EU4096};
use ripemd160::{Digest, Ripemd160};
use sha2::Sha256;
use std::cmp;
use std::io::Write;
use std::io::{self, Read};
use std::u64;

/// Implementation of a pre-compiled contract.
pub trait PrecompiledContract: Send + Sync {
    /// Return required gas for contract call.
    fn required_gas(&self, input: &[u8]) -> u64;

    /// Get the output from the pre-compiled contract.
    fn run(&self, input: &[u8]) -> Result<Vec<u8>, err::Error>;
}

/// Function get returns a pre-compiled contract by given address.
pub fn get(address: Address) -> Box<PrecompiledContract> {
    match U256::from(H256::from(address)).low_u64() {
        0x01 => Box::new(EcRecover {}) as Box<PrecompiledContract>,
        0x02 => Box::new(SHA256Hash {}) as Box<PrecompiledContract>,
        0x03 => Box::new(RIPEMD160Hash {}) as Box<PrecompiledContract>,
        0x04 => Box::new(DataCopy {}) as Box<PrecompiledContract>,
        0x05 => Box::new(BigModExp {}) as Box<PrecompiledContract>,
        0x06 => Box::new(Bn256Add {}) as Box<PrecompiledContract>,
        0x07 => Box::new(Bn256ScalarMul {}) as Box<PrecompiledContract>,
        0x08 => Box::new(Bn256Pairing {}) as Box<PrecompiledContract>,
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
const G_MOD_EXP_QUADCOEFF_DIV: u64 = 20; // Divisor for the quadratic particle of the big int modular exponentiation
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

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
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
            let data = state::hashlib::summary(&public.0);
            output.write_all(&[0; 12])?;
            output.write_all(&data[12..data.len()])?;
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

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
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

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
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

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(i.into())
    }
}

/// BigModExp implements a native big integer exponential modular operation.
/// Input in the following format:
///   <length_of_BASE> <length_of_EXPONENT> <length_of_MODULUS> <BASE> <EXPONENT> <MODULUS>
///
/// See: https://github.com/ethereum/EIPs/blob/master/EIPS/eip-198.md
pub struct BigModExp {}

impl BigModExp {
    fn get_len(&self, r: &mut impl Read) -> EU256 {
        let mut buf = vec![0; 32];
        r.read_exact(&mut buf[..]).unwrap(); // unwrap here is ok
        EU256::from_big_endian(&buf[..]).unwrap()
    }

    fn get_num(&self, r: &mut impl Read, len: usize) -> (EU4096, Vec<u8>) {
        let mut buf = vec![0; len];
        r.read_exact(&mut buf[..]).unwrap();
        (EU4096::from_big_endian(&buf[..]).unwrap(), buf.to_vec())
    }

    fn adjusted_length_of_exponent(&self, len: u64, exponent: EU256) -> u64 {
        let bit_index = if exponent.is_zero() {
            0
        } else {
            u64::from(255 - exponent.leading_zeros())
        };
        if len <= 32 {
            bit_index
        } else {
            8 * (len - 32) + bit_index
        }
    }

    fn mult_complexity(&self, x: u64) -> u64 {
        if x <= 64 {
            x * x
        } else if x <= 1024 {
            (x * x) / 4 + 96 * x - 3072
        } else {
            (x * x) / 16 + 480 * x - 199_680
        }
    }

    // Calculate modexp: left-to-right binary exponentiation to keep multiplicands lower
    fn alu(&self, mut base: EU4096, exp: Vec<u8>, modulus: EU4096) -> EU4096 {
        if modulus <= EU4096::one() {
            return EU4096::zero();
        }
        let mut exp = exp.into_iter().skip_while(|d| *d == 0).peekable();
        if exp.peek().is_none() {
            return EU4096::one();
        }
        if base.is_zero() {
            return EU4096::zero();
        }
        base %= &modulus;
        if base.is_zero() {
            return EU4096::zero();
        }
        // Left-to-right binary exponentiation (Handbook of Applied Cryptography - Algorithm 14.79).
        // http://www.cacr.math.uwaterloo.ca/hac/about/chap14.pdf
        let mut result = EU4096::one();
        for digit in exp {
            let mut mask = 1 << (8 - 1);
            for _ in 0..8 {
                result = &result * &result % &modulus;
                if digit & mask > 0 {
                    result = result * &base % &modulus;
                }
                mask >>= 1;
            }
        }
        result
    }
}

impl PrecompiledContract for BigModExp {
    // Returns floor(mult_complexity(max(length_of_MODULUS, length_of_BASE)) * max(ADJUSTED_EXPONENT_LENGTH, 1) / GQUADDIVISOR)
    fn required_gas(&self, input: &[u8]) -> u64 {
        let mut reader = input.chain(io::repeat(0));
        let length_of_base = self.get_len(&mut reader);
        let length_of_exponent = self.get_len(&mut reader);
        let length_of_modulus = self.get_len(&mut reader);
        if length_of_modulus.is_zero() && length_of_base.is_zero() {
            return 0;
        }
        let lim = EU256::from(512u32); // Limit to U4096
        if length_of_base > lim || length_of_modulus > lim || length_of_exponent > lim {
            return u64::MAX;
        }
        let (length_of_base, length_of_exponent, length_of_modulus) =
            (length_of_base.0[0], length_of_exponent.0[0], length_of_modulus.0[0]);

        let m = std::cmp::max(length_of_modulus, length_of_base);
        let mult_c = self.mult_complexity(m);

        let exponent = if length_of_base + 96 >= input.len() as u64 {
            EU256::zero()
        } else {
            let mut buf = [0; 32];
            let mut reader = input[(96 + length_of_base as usize)..].chain(io::repeat(0));
            let len = cmp::min(length_of_exponent, 32) as usize;
            reader.read_exact(&mut buf[(32 - len)..]).unwrap(); // unwrap here is ok
            EU256::from_big_endian(&buf[..]).unwrap()
        };
        let adjusted_length_of_exponent = self.adjusted_length_of_exponent(length_of_exponent, exponent);
        let (gas, overflow) = mult_c.overflowing_mul(cmp::max(adjusted_length_of_exponent, 1));
        if overflow {
            return std::u64::MAX;
        }
        gas / G_MOD_EXP_QUADCOEFF_DIV as u64
    }

    /// Returns (BASE**EXPONENT) % MODULUS
    fn run(&self, input: &[u8]) -> Result<Vec<u8>, err::Error> {
        let mut reader = input.chain(io::repeat(0));
        let length_of_base = self.get_len(&mut reader).0[0];
        let length_of_exponent = self.get_len(&mut reader).0[0];
        let length_of_modulus = self.get_len(&mut reader).0[0];
        if length_of_modulus.is_zero() && length_of_base.is_zero() {
            return Ok(vec![0; length_of_modulus as usize]);
        }
        let (base, _) = self.get_num(&mut reader, length_of_base as usize);
        let (_, exponent_raw) = self.get_num(&mut reader, length_of_exponent as usize);
        let (modulus, _) = self.get_num(&mut reader, length_of_modulus as usize);
        let r = self.alu(base, exponent_raw, modulus);
        let mut buf = vec![0; 512];
        r.into_big_endian(&mut buf).unwrap();
        let a = buf[512 - length_of_modulus as usize..].to_vec();
        Ok(a)
    }
}

/// Bn256Add implements a native elliptic curve point addition.
pub struct Bn256Add {}

impl PrecompiledContract for Bn256Add {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_BN256_ADD
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, err::Error> {
        Err(err::Error::Str("Not implemented!".into()))
    }
}

/// Bn256ScalarMul implements a native elliptic curve point addition.
pub struct Bn256ScalarMul {}

impl PrecompiledContract for Bn256ScalarMul {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_BN256_SCALAR_MUL
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, err::Error> {
        Err(err::Error::Str("Not implemented!".into()))
    }
}

/// Bn256Pairing implements a pairing pre-compile for the bn256 curve
pub struct Bn256Pairing {}

impl PrecompiledContract for Bn256Pairing {
    fn required_gas(&self, i: &[u8]) -> u64 {
        G_BN256_PARING_BASE + (i.len() as u64 / 192 * G_BN256_PARING_PER_POINT)
    }

    fn run(&self, _: &[u8]) -> Result<Vec<u8>, err::Error> {
        Err(err::Error::Str("Not implemented!".into()))
    }
}
