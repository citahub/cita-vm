use std::error;
use std::fmt;
use std::io;

use crate::evm;
use crate::state::Error as StateError;

#[derive(Debug)]
pub enum Error {
    Evm(evm::Error),
    Secp256k1(secp256k1::Error),
    State(StateError),
    IO(io::Error),
    Str(String),
    NotEnoughBaseGas,
    NotEnoughBalance,
    InvalidNonce,
    ContractAlreadyExist,
    ExccedMaxCodeSize,
    ExccedMaxBlockGasLimit,
    ExccedMaxCallDepth,
    CreateInStaticCall,
}

impl error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Evm(e) => write!(f, "{}", e),
            Error::Secp256k1(e) => write!(f, "{:?}", e),
            Error::State(e) => write!(f, "{}", e),
            Error::IO(e) => write!(f, "{:?}", e),
            Error::Str(e) => write!(f, "{:?}", e),
            Error::NotEnoughBaseGas => write!(f, "NotEnoughBaseGas"),
            Error::NotEnoughBalance => write!(f, "NotEnoughBalance"),
            Error::InvalidNonce => write!(f, "InvalidNonce"),
            Error::ContractAlreadyExist => write!(f, "ContractAlreadyExist"),
            Error::ExccedMaxCodeSize => write!(f, "ExccedMaxCodeSize"),
            Error::ExccedMaxBlockGasLimit => write!(f, "ExccedMaxBlockGasLimit"),
            Error::ExccedMaxCallDepth => write!(f, "ExccedMaxCallDepth"),
            Error::CreateInStaticCall => write!(f, "CreateInStaticCall"),
        }
    }
}

impl From<evm::Error> for Error {
    fn from(error: evm::Error) -> Self {
        Error::Evm(error)
    }
}

impl From<StateError> for Error {
    fn from(error: StateError) -> Self {
        Error::State(error)
    }
}

impl From<secp256k1::Error> for Error {
    fn from(error: secp256k1::Error) -> Self {
        Error::Secp256k1(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IO(error)
    }
}
