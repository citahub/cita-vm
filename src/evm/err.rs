use std::error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    OutOfBounds,
    OutOfGas,
    OutOfStack,
    OutOfCode,
    OutOfData,
    MutableCallInStaticContext,
    InvalidOpcode,
    CallError,
    ExccedMaxCodeSize,
    InvalidJumpDestination,
    Internal(String),
}

impl error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::OutOfBounds => return write!(f, "OutOfBounds"),
            Error::OutOfGas => return write!(f, "OutOfGas"),
            Error::OutOfStack => return write!(f, "OutOfStack"),
            Error::OutOfCode => return write!(f, "OutOfCode"),
            Error::OutOfData => return write!(f, "OutOfData"),
            Error::MutableCallInStaticContext => return write!(f, "MutableCallInStaticContext"),
            Error::InvalidOpcode => return write!(f, "InvalidOpcode"),
            Error::CallError => return write!(f, "CallError"),
            Error::ExccedMaxCodeSize => return write!(f, "ExccedMaxCodeSize"),
            Error::InvalidJumpDestination => return write!(f, "InvalidJumpDestination"),
            Error::Internal(err) => return write!(f, "Internal error {}", err),
        };
    }
}
