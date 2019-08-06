use std::error;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CallError,
    ExccedMaxCodeSize,
    InvalidJumpDestination,
    InvalidOpcode,
    MutableCallInStaticContext,
    OutOfBounds,
    OutOfCode,
    OutOfData,
    OutOfGas,
    OutOfStack,
}

impl error::Error for Error {}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CallError => return write!(f, "CallError"),
            Error::ExccedMaxCodeSize => return write!(f, "ExccedMaxCodeSize"),
            Error::InvalidJumpDestination => return write!(f, "InvalidJumpDestination"),
            Error::InvalidOpcode => return write!(f, "InvalidOpcode"),
            Error::MutableCallInStaticContext => return write!(f, "MutableCallInStaticContext"),
            Error::OutOfBounds => return write!(f, "OutOfBounds"),
            Error::OutOfCode => return write!(f, "OutOfCode"),
            Error::OutOfData => return write!(f, "OutOfData"),
            Error::OutOfGas => return write!(f, "OutOfGas"),
            Error::OutOfStack => return write!(f, "OutOfStack"),
        };
    }
}
