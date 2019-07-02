use std::error;
use std::fmt;

use ckb_vm;

use crate::evm;
use crate::state::Error as StateError;

#[derive(Debug)]
pub enum Error {
    ContractAlreadyExist,
    CreateInStaticCall,
    ExccedMaxBlockGasLimit,
    ExccedMaxCallDepth,
    ExccedMaxCodeSize,
    ExitCodeError,
    EVM(evm::Error),
    InvalidNonce,
    NotEnoughBalance,
    NotEnoughBaseGas,
    RISCV(ckb_vm::Error),
    Secp256k1(secp256k1::Error),
    State(StateError),
}

impl error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ContractAlreadyExist => return write!(f, "ContractAlreadyExist"),
            Error::CreateInStaticCall => return write!(f, "CreateInStaticCall"),
            Error::ExccedMaxBlockGasLimit => return write!(f, "ExccedMaxBlockGasLimit"),
            Error::ExccedMaxCallDepth => return write!(f, "ExccedMaxCallDepth"),
            Error::ExccedMaxCodeSize => return write!(f, "ExccedMaxCodeSize"),
            Error::ExitCodeError => return write!(f, "ExitCodeError"),
            Error::EVM(e) => return write!(f, "{}", e),
            Error::InvalidNonce => return write!(f, "InvalidNonce"),
            Error::NotEnoughBalance => return write!(f, "NotEnoughBalance"),
            Error::NotEnoughBaseGas => return write!(f, "NotEnoughBaseGas"),
            Error::RISCV(e) => return write!(f, "{:?}", e),
            Error::Secp256k1(e) => return write!(f, "{:?}", e),
            Error::State(e) => return write!(f, "{}", e),
        };
    }
}

impl From<evm::Error> for Error {
    fn from(error: evm::Error) -> Self {
        Error::EVM(error)
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

impl From<ckb_vm::Error> for Error {
    fn from(error: ckb_vm::Error) -> Self {
        Error::RISCV(error)
    }
}
