use std::fmt::{Display, Formatter, Result};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum OpCode {
    // 0x0 range - arithmetic ops.
    STOP = 0x00,
    ADD = 0x01,
    MUL = 0x02,
    SUB = 0x03,
    DIV = 0x04,
    SDIV = 0x05,
    MOD = 0x06,
    SMOD = 0x07,
    ADDMOD = 0x08,
    MULMOD = 0x09,
    EXP = 0x0a,
    SIGNEXTEND = 0x0b,

    // 0x10 range - comparison ops.
    LT = 0x10,
    GT = 0x11,
    SLT = 0x12,
    SGT = 0x13,
    EQ = 0x14,
    ISZERO = 0x15,
    AND = 0x16,
    OR = 0x17,
    XOR = 0x18,
    NOT = 0x19,
    BYTE = 0x1a,
    SHL = 0x1b,
    SHR = 0x1c,
    SAR = 0x1d,

    SHA3 = 0x20,

    // 0x30 range - closure state.
    ADDRESS = 0x30,
    BALANCE = 0x31,
    ORIGIN = 0x32,
    CALLER = 0x33,
    CALLVALUE = 0x34,
    CALLDATALOAD = 0x35,
    CALLDATASIZE = 0x36,
    CALLDATACOPY = 0x37,
    CODESIZE = 0x38,
    CODECOPY = 0x39,
    GASPRICE = 0x3a,
    EXTCODESIZE = 0x3b,
    EXTCODECOPY = 0x3c,
    RETURNDATASIZE = 0x3d,
    RETURNDATACOPY = 0x3e,
    EXTCODEHASH = 0x3f,
}

impl Display for OpCode {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            OpCode::STOP => write!(f, "STOP"),
            _ => write!(f, "opcode"),
        }
    }
}

pub fn u8_2_opcode(n: u8) -> OpCode {
    match n {
        0x00 => OpCode::STOP,
        0x01 => OpCode::ADD,
        0x02 => OpCode::MUL,
        0x03 => OpCode::SUB,
        0x04 => OpCode::DIV,
        0x05 => OpCode::SDIV,
        0x06 => OpCode::MOD,
        0x07 => OpCode::SMOD,
        0x08 => OpCode::ADDMOD,
        0x09 => OpCode::MULMOD,
        0x0a => OpCode::EXP,
        0x0b => OpCode::SIGNEXTEND,
        0x10 => OpCode::LT,
        0x11 => OpCode::GT,
        0x12 => OpCode::SLT,
        0x13 => OpCode::SGT,
        0x14 => OpCode::EQ,
        0x15 => OpCode::ISZERO,
        0x16 => OpCode::AND,
        0x17 => OpCode::OR,
        0x18 => OpCode::XOR,
        0x19 => OpCode::NOT,
        0x1a => OpCode::BYTE,
        0x1b => OpCode::SHL,
        0x1c => OpCode::SHR,
        0x1d => OpCode::SAR,
        0x20 => OpCode::SHA3,
        0x30 => OpCode::ADDRESS,
        0x31 => OpCode::BALANCE,
        0x32 => OpCode::ORIGIN,
        0x33 => OpCode::CALLER,
        0x34 => OpCode::CALLVALUE,
        0x35 => OpCode::CALLDATALOAD,
        0x36 => OpCode::CALLDATASIZE,
        0x37 => OpCode::CALLDATACOPY,
        0x38 => OpCode::CODESIZE,
        0x39 => OpCode::CODECOPY,
        0x3a => OpCode::GASPRICE,
        0x3b => OpCode::EXTCODESIZE,
        0x3c => OpCode::EXTCODECOPY,
        0x3d => OpCode::RETURNDATASIZE,
        0x3e => OpCode::RETURNDATACOPY,
        0x3f => OpCode::EXTCODEHASH,
        _ => panic!("invalid u8 to opcode"),
    }
}
