use super::err;

pub trait PrecompiledContract {
    fn required_gas(input: &[u8]) -> u64;
    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error>;
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
const G_BN256S_CALAR_MUL: u64 = 40000; // Gas needed for an elliptic curve scalar multiplication
const G_BN256_PARING_BASE: u64 = 100_000; // Base price for an elliptic curve pairing check
const G_BN256_PARING_PER_POINT: u64 = 80000; // Per-point price for an elliptic curve pairing check

// ECRECOVER implemented as a native contract.
pub struct Ecrecover {}

impl PrecompiledContract for Ecrecover {
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}

// SHA256 implemented as a native contract.
pub struct SHA256Hash {}

impl PrecompiledContract for SHA256Hash {
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}

// RIPEMD160 implemented as a native contract.
pub struct RIPEMD160Hash {}

impl PrecompiledContract for RIPEMD160Hash {
    #[allow(unused_variables)]
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}

// bigModExp implements a native big integer exponential modular operation.
pub struct BigModExp {}

impl PrecompiledContract for BigModExp {
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}

// bn256Add implements a native elliptic curve point addition.
pub struct Bn256Add {}

impl PrecompiledContract for Bn256Add {
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}

// Bn256ScalarMul implements a native elliptic curve point addition.
pub struct Bn256ScalarMul {}

impl PrecompiledContract for Bn256ScalarMul {
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}

// Bn256Pairing implements a pairing pre-compile for the bn256 curve
pub struct Bn256Pairing {}

impl PrecompiledContract for Bn256Pairing {
    fn required_gas(input: &[u8]) -> u64 {
        G_ECRECOVER
    }

    fn run(input: &[u8]) -> Result<Vec<u8>, err::Error> {
        Ok(vec![])
    }
}
