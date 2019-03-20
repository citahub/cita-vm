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
//!
//! Contracts 5 to 8 reference parity-ethereum's code.
use super::err;
use bn::{pairing, AffineG1, AffineG2, Fq, Fq2, Group, Gt, G1, G2};
use byteorder::{BigEndian, ByteOrder};
use ethereum_types::{Address, H256, H512, U256};
use num::{BigUint, One, Zero};
use ripemd160::{Digest, Ripemd160};
use sha2::Sha256;
use std::io;
use std::io::{Read, Write};

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

// Calculate modexp: left-to-right binary exponentiation to keep multiplicands lower
fn modexp(mut base: BigUint, exp: Vec<u8>, modulus: BigUint) -> BigUint {
    const BITS_PER_DIGIT: usize = 8;
    if modulus <= BigUint::one() {
        return BigUint::zero();
    }
    let mut exp = exp.into_iter().skip_while(|d| *d == 0).peekable();
    if exp.peek().is_none() {
        return BigUint::one();
    }
    if base.is_zero() {
        return BigUint::zero();
    }
    base %= &modulus;
    if base.is_zero() {
        return BigUint::zero();
    }
    // Left-to-right binary exponentiation (Handbook of Applied Cryptography - Algorithm 14.79).
    // http://www.cacr.math.uwaterloo.ca/hac/about/chap14.pdf
    let mut result = BigUint::one();
    for digit in exp {
        let mut mask = 1 << (BITS_PER_DIGIT - 1);
        for _ in 0..BITS_PER_DIGIT {
            result = &result * &result % &modulus;
            if digit & mask > 0 {
                result = result * &base % &modulus;
            }
            mask >>= 1;
        }
    }
    result
}

/// BigModExp implements a native big integer exponential modular operation.
pub struct BigModExp {}

impl BigModExp {
    fn adjusted_exp_len(len: u64, exp_low: U256) -> u64 {
        let bit_index = if exp_low.is_zero() {
            0
        } else {
            u64::from(255 - exp_low.leading_zeros())
        };
        if len <= 32 {
            bit_index
        } else {
            8 * (len - 32) + bit_index
        }
    }

    fn mult_complexity(x: u64) -> u64 {
        match x {
            x if x <= 64 => x * x,
            x if x <= 1024 => (x * x) / 4 + 96 * x - 3072,
            x => (x * x) / 16 + 480 * x - 199_680,
        }
    }
}

impl PrecompiledContract for BigModExp {
    fn required_gas(&self, input: &[u8]) -> u64 {
        let mut reader = input.chain(io::repeat(0));
        let mut buf = [0; 32];

        // Read lengths as U256 here for accurate gas calculation.
        let mut read_len = || {
            reader
                .read_exact(&mut buf[..])
                .expect("reading from zero-extended memory cannot fail; qed");
            U256::from(H256::from(&buf[..]))
        };
        let base_len = read_len();
        let exp_len = read_len();
        let mod_len = read_len();

        if mod_len.is_zero() && base_len.is_zero() {
            return 0;
        }

        let max_len = U256::from(u32::max_value() / 2);
        if base_len > max_len || mod_len > max_len || exp_len > max_len {
            return std::u64::MAX;
        }
        let (base_len, exp_len, mod_len) = (base_len.low_u64(), exp_len.low_u64(), mod_len.low_u64());

        let m = std::cmp::max(mod_len, base_len);
        // Read first 32-byte word of the exponent.
        let exp_low = if base_len + 96 >= input.len() as u64 {
            U256::zero()
        } else {
            let mut buf = [0; 32];
            let mut reader = input[(96 + base_len as usize)..].chain(io::repeat(0));
            let len = std::cmp::min(exp_len, 32) as usize;
            reader
                .read_exact(&mut buf[(32 - len)..])
                .expect("reading from zero-extended memory cannot fail; qed");
            U256::from(H256::from(&buf[..]))
        };

        let adjusted_exp_len = Self::adjusted_exp_len(exp_len, exp_low);
        let (gas, overflow) = Self::mult_complexity(m).overflowing_mul(std::cmp::max(adjusted_exp_len, 1));
        if overflow {
            return std::u64::MAX;
        }
        gas / G_MOD_EXP_QUADCOEFF_DIV as u64
    }

    fn run(&self, input: &[u8]) -> Result<Vec<u8>, err::Error> {
        let mut output: Vec<u8> = Vec::new();

        let mut reader = input.chain(io::repeat(0));
        let mut buf = [0; 32];

        // Read lengths as usize.
        // ignoring the first 24 bytes might technically lead us to fall out of consensus,
        // but so would running out of addressable memory!
        let mut read_len = |reader: &mut io::Chain<&[u8], io::Repeat>| {
            reader
                .read_exact(&mut buf[..])
                .expect("reading from zero-extended memory cannot fail; qed");
            BigEndian::read_u64(&buf[24..]) as usize
        };

        let base_len = read_len(&mut reader);
        let exp_len = read_len(&mut reader);
        let mod_len = read_len(&mut reader);

        // Gas formula allows arbitrary large exp_len when base and modulus are empty, so we need to handle empty base first.
        let r = if base_len == 0 && mod_len == 0 {
            BigUint::zero()
        } else {
            // Read the numbers themselves.
            let mut buf = vec![0; std::cmp::max(mod_len, std::cmp::max(base_len, exp_len))];
            let mut read_num = |reader: &mut io::Chain<&[u8], io::Repeat>, len: usize| {
                reader
                    .read_exact(&mut buf[..len])
                    .expect("reading from zero-extended memory cannot fail; qed");
                BigUint::from_bytes_be(&buf[..len])
            };

            let base = read_num(&mut reader, base_len);

            let mut exp_buf = vec![0; exp_len];
            reader
                .read_exact(&mut exp_buf[..exp_len])
                .expect("reading from zero-extended memory cannot fail; qed");

            let modulus = read_num(&mut reader, mod_len);

            modexp(base, exp_buf, modulus)
        };

        // Write output to given memory, left padded and same length as the modulus.
        let bytes = r.to_bytes_be();

        // Always true except in the case of zero-length modulus, which leads to
        // output of length and value 1.
        if bytes.len() <= mod_len {
            let pre = vec![0u8; mod_len - bytes.len()];
            output.write_all(&pre)?;
            output.write_all(&bytes)?;
        }
        Ok(output)
    }
}

fn read_fr(reader: &mut io::Chain<&[u8], io::Repeat>) -> Result<bn::Fr, err::Error> {
    let mut buf = [0u8; 32];

    reader
        .read_exact(&mut buf[..])
        .expect("reading from zero-extended memory cannot fail; qed");
    bn::Fr::from_slice(&buf[0..32]).map_err(|_| err::Error::Str(String::from("Invalid field element")))
}

fn read_point(reader: &mut io::Chain<&[u8], io::Repeat>) -> Result<bn::G1, err::Error> {
    let mut buf = [0u8; 32];

    reader
        .read_exact(&mut buf[..])
        .expect("reading from zero-extended memory cannot fail; qed");
    let px = Fq::from_slice(&buf[0..32]).map_err(|_| err::Error::Str(String::from("Invalid point x coordinate")))?;

    reader
        .read_exact(&mut buf[..])
        .expect("reading from zero-extended memory cannot fail; qed");
    let py = Fq::from_slice(&buf[0..32]).map_err(|_| err::Error::Str(String::from("Invalid point y coordinate")))?;
    Ok(if px == Fq::zero() && py == Fq::zero() {
        G1::zero()
    } else {
        AffineG1::new(px, py)
            .map_err(|_| err::Error::Str(String::from("Invalid curve point")))?
            .into()
    })
}

/// Bn256Add implements a native elliptic curve point addition.
pub struct Bn256Add {}

impl PrecompiledContract for Bn256Add {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_BN256_ADD
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
        let mut output: Vec<u8> = Vec::new();
        let mut padded_input = i.chain(io::repeat(0));
        let p1 = read_point(&mut padded_input)?;
        let p2 = read_point(&mut padded_input)?;

        let mut write_buf = [0u8; 64];
        if let Some(sum) = AffineG1::from_jacobian(p1 + p2) {
            // Point not at infinity
            sum.x()
                .to_big_endian(&mut write_buf[0..32])
                .expect("Cannot fail since 0..32 is 32-byte length");
            sum.y()
                .to_big_endian(&mut write_buf[32..64])
                .expect("Cannot fail since 32..64 is 32-byte length");
        }
        output.write_all(&write_buf)?;
        Ok(output)
    }
}

/// Bn256ScalarMul implements a native elliptic curve point addition.
pub struct Bn256ScalarMul {}

impl PrecompiledContract for Bn256ScalarMul {
    fn required_gas(&self, _: &[u8]) -> u64 {
        G_BN256_SCALAR_MUL
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
        let mut output: Vec<u8> = Vec::new();
        let mut padded_input = i.chain(io::repeat(0));
        let p = read_point(&mut padded_input)?;
        let fr = read_fr(&mut padded_input)?;

        let mut write_buf = [0u8; 64];
        if let Some(sum) = AffineG1::from_jacobian(p * fr) {
            // Point not at infinity
            sum.x()
                .to_big_endian(&mut write_buf[0..32])
                .expect("Cannot fail since 0..32 is 32-byte length");
            sum.y()
                .to_big_endian(&mut write_buf[32..64])
                .expect("Cannot fail since 32..64 is 32-byte length");;
        }
        output.write_all(&write_buf)?;
        Ok(output)
    }
}

/// Bn256Pairing implements a pairing pre-compile for the bn256 curve
pub struct Bn256Pairing {}

impl PrecompiledContract for Bn256Pairing {
    fn required_gas(&self, i: &[u8]) -> u64 {
        G_BN256_PARING_BASE + (i.len() as u64 / 192 * G_BN256_PARING_PER_POINT)
    }

    fn run(&self, i: &[u8]) -> Result<Vec<u8>, err::Error> {
        if i.len() % 192 != 0 {
            return Err(err::Error::Str(String::from(
                "Invalid input length, must be multiple of 192 (3 * (32*2))",
            )));
        }
        let mut output: Vec<u8> = Vec::new();

        let elements = i.len() / 192; // (a, b_a, b_b - each 64-byte affine coordinates)
        let ret_val = if i.is_empty() {
            U256::one()
        } else {
            let mut vals = Vec::new();
            for idx in 0..elements {
                let a_x = Fq::from_slice(&i[idx * 192..idx * 192 + 32])
                    .map_err(|_| err::Error::Str(String::from("Invalid a argument x coordinate")))?;

                let a_y = Fq::from_slice(&i[idx * 192 + 32..idx * 192 + 64])
                    .map_err(|_| err::Error::Str(String::from("Invalid a argument y coordinate")))?;

                let b_a_y = Fq::from_slice(&i[idx * 192 + 64..idx * 192 + 96])
                    .map_err(|_| err::Error::Str(String::from("Invalid b argument imaginary coeff x coordinate")))?;

                let b_a_x = Fq::from_slice(&i[idx * 192 + 96..idx * 192 + 128])
                    .map_err(|_| err::Error::Str(String::from("Invalid b argument imaginary coeff y coordinate")))?;

                let b_b_y = Fq::from_slice(&i[idx * 192 + 128..idx * 192 + 160])
                    .map_err(|_| err::Error::Str(String::from("Invalid b argument real coeff x coordinate")))?;

                let b_b_x = Fq::from_slice(&i[idx * 192 + 160..idx * 192 + 192])
                    .map_err(|_| err::Error::Str(String::from("Invalid b argument real coeff y coordinate")))?;

                let b_a = Fq2::new(b_a_x, b_a_y);
                let b_b = Fq2::new(b_b_x, b_b_y);
                let b = if b_a.is_zero() && b_b.is_zero() {
                    G2::zero()
                } else {
                    G2::from(
                        AffineG2::new(b_a, b_b)
                            .map_err(|_| err::Error::Str(String::from("Invalid b argument - not on curve")))?,
                    )
                };
                let a = if a_x.is_zero() && a_y.is_zero() {
                    G1::zero()
                } else {
                    G1::from(
                        AffineG1::new(a_x, a_y)
                            .map_err(|_| err::Error::Str(String::from("Invalid a argument - not on curve")))?,
                    )
                };
                vals.push((a, b));
            }

            let mul = vals.into_iter().fold(Gt::one(), |s, (a, b)| s * pairing(a, b));

            if mul == Gt::one() {
                U256::one()
            } else {
                U256::zero()
            }
        };

        let mut buf = [0u8; 32];
        ret_val.to_big_endian(&mut buf);
        output.write_all(&buf)?;
        Ok(output)
    }
}
