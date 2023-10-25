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
    StackUnderflow,
}

impl error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::OutOfBounds => write!(f, "OutOfBounds"),
            Error::OutOfGas => write!(f, "OutOfGas"),
            Error::OutOfStack => write!(f, "OutOfStack"),
            Error::OutOfCode => write!(f, "OutOfCode"),
            Error::OutOfData => write!(f, "OutOfData"),
            Error::MutableCallInStaticContext => write!(f, "MutableCallInStaticContext"),
            Error::InvalidOpcode => write!(f, "InvalidOpcode"),
            Error::CallError => write!(f, "CallError"),
            Error::ExccedMaxCodeSize => write!(f, "ExccedMaxCodeSize"),
            Error::InvalidJumpDestination => write!(f, "InvalidJumpDestination"),
            Error::Internal(err) => write!(f, "Internal error {}", err),
            Error::StackUnderflow => write!(f, "StackUnderflow"),
        }
    }
}
