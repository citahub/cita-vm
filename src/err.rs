#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    Evm(evm::err::Error),
    State(state::err::Error),
    NotEnoughBaseGas,
    NotEnoughBalance,
    InvalidNonce,
    ContractAlreadyExist,
    ExccedMaxCodeSize,
    ExccedMaxBlockGasLimit,
    ExccedMaxCallDepth,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Evm(e) => return write!(f, "{}", e),
            Error::State(e) => return write!(f, "{}", e),
            Error::NotEnoughBaseGas => return write!(f, "NotEnoughBaseGas"),
            Error::NotEnoughBalance => return write!(f, "NotEnoughBalance"),
            Error::InvalidNonce => return write!(f, "InvalidNonce"),
            Error::ContractAlreadyExist => return write!(f, "ContractAlreadyExist"),
            Error::ExccedMaxCodeSize => return write!(f, "ExccedMaxCodeSize"),
            Error::ExccedMaxBlockGasLimit => return write!(f, "ExccedMaxBlockGasLimit"),
            Error::ExccedMaxCallDepth => return write!(f, "ExccedMaxCallDepth"),
        };
    }
}

impl From<evm::err::Error> for Error {
    fn from(error: evm::err::Error) -> Self {
        Error::Evm(error)
    }
}

impl From<state::err::Error> for Error {
    fn from(error: state::err::Error) -> Self {
        Error::State(error)
    }
}
