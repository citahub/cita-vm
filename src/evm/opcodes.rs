use std::fmt;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum OpCode {
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
    BLOCKHASH = 0x40,
    COINBASE = 0x41,
    TIMESTAMP = 0x42,
    NUMBER = 0x43,
    DIFFICULTY = 0x44,
    GASLIMIT = 0x45,
    CHAINID = 0x46,
    SELFBALANCE = 0x47,
    BASEFEE = 0x48,
    POP = 0x50,
    MLOAD = 0x51,
    MSTORE = 0x52,
    MSTORE8 = 0x53,
    SLOAD = 0x54,
    SSTORE = 0x55,
    JUMP = 0x56,
    JUMPI = 0x57,
    PC = 0x58,
    MSIZE = 0x59,
    GAS = 0x5a,
    JUMPDEST = 0x5b,
    PUSH1 = 0x60,
    PUSH2 = 0x61,
    PUSH3 = 0x62,
    PUSH4 = 0x63,
    PUSH5 = 0x64,
    PUSH6 = 0x65,
    PUSH7 = 0x66,
    PUSH8 = 0x67,
    PUSH9 = 0x68,
    PUSH10 = 0x69,
    PUSH11 = 0x6a,
    PUSH12 = 0x6b,
    PUSH13 = 0x6c,
    PUSH14 = 0x6d,
    PUSH15 = 0x6e,
    PUSH16 = 0x6f,
    PUSH17 = 0x70,
    PUSH18 = 0x71,
    PUSH19 = 0x72,
    PUSH20 = 0x73,
    PUSH21 = 0x74,
    PUSH22 = 0x75,
    PUSH23 = 0x76,
    PUSH24 = 0x77,
    PUSH25 = 0x78,
    PUSH26 = 0x79,
    PUSH27 = 0x7a,
    PUSH28 = 0x7b,
    PUSH29 = 0x7c,
    PUSH30 = 0x7d,
    PUSH31 = 0x7e,
    PUSH32 = 0x7f,
    DUP1 = 0x80,
    DUP2 = 0x81,
    DUP3 = 0x82,
    DUP4 = 0x83,
    DUP5 = 0x84,
    DUP6 = 0x85,
    DUP7 = 0x86,
    DUP8 = 0x87,
    DUP9 = 0x88,
    DUP10 = 0x89,
    DUP11 = 0x8a,
    DUP12 = 0x8b,
    DUP13 = 0x8c,
    DUP14 = 0x8d,
    DUP15 = 0x8e,
    DUP16 = 0x8f,
    SWAP1 = 0x90,
    SWAP2 = 0x91,
    SWAP3 = 0x92,
    SWAP4 = 0x93,
    SWAP5 = 0x94,
    SWAP6 = 0x95,
    SWAP7 = 0x96,
    SWAP8 = 0x97,
    SWAP9 = 0x98,
    SWAP10 = 0x99,
    SWAP11 = 0x9a,
    SWAP12 = 0x9b,
    SWAP13 = 0x9c,
    SWAP14 = 0x9d,
    SWAP15 = 0x9e,
    SWAP16 = 0x9f,
    LOG0 = 0xa0,
    LOG1 = 0xa1,
    LOG2 = 0xa2,
    LOG3 = 0xa3,
    LOG4 = 0xa4,
    CREATE = 0xf0,
    CALL = 0xf1,
    CALLCODE = 0xf2,
    RETURN = 0xf3,
    DELEGATECALL = 0xf4,
    CREATE2 = 0xf5,
    REVERT = 0xfd,
    STATICCALL = 0xfa,
    SELFDESTRUCT = 0xff,
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OpCode::STOP => write!(f, "STOP"),
            OpCode::ADD => write!(f, "ADD"),
            OpCode::MUL => write!(f, "MUL"),
            OpCode::SUB => write!(f, "SUB"),
            OpCode::DIV => write!(f, "DIV"),
            OpCode::SDIV => write!(f, "SDIV"),
            OpCode::MOD => write!(f, "MOD"),
            OpCode::SMOD => write!(f, "SMOD"),
            OpCode::ADDMOD => write!(f, "ADDMOD"),
            OpCode::MULMOD => write!(f, "MULMOD"),
            OpCode::EXP => write!(f, "EXP"),
            OpCode::SIGNEXTEND => write!(f, "SIGNEXTEND"),
            OpCode::LT => write!(f, "LT"),
            OpCode::GT => write!(f, "GT"),
            OpCode::SLT => write!(f, "SLT"),
            OpCode::SGT => write!(f, "SGT"),
            OpCode::EQ => write!(f, "EQ"),
            OpCode::ISZERO => write!(f, "ISZERO"),
            OpCode::AND => write!(f, "AND"),
            OpCode::OR => write!(f, "OR"),
            OpCode::XOR => write!(f, "XOR"),
            OpCode::NOT => write!(f, "NOT"),
            OpCode::BYTE => write!(f, "BYTE"),
            OpCode::SHL => write!(f, "SHL"),
            OpCode::SHR => write!(f, "SHR"),
            OpCode::SAR => write!(f, "SAR"),
            OpCode::SHA3 => write!(f, "SHA3"),
            OpCode::ADDRESS => write!(f, "ADDRESS"),
            OpCode::BALANCE => write!(f, "BALANCE"),
            OpCode::ORIGIN => write!(f, "ORIGIN"),
            OpCode::CALLER => write!(f, "CALLER"),
            OpCode::CALLVALUE => write!(f, "CALLVALUE"),
            OpCode::CALLDATALOAD => write!(f, "CALLDATALOAD"),
            OpCode::CALLDATASIZE => write!(f, "CALLDATASIZE"),
            OpCode::CALLDATACOPY => write!(f, "CALLDATACOPY"),
            OpCode::CODESIZE => write!(f, "CODESIZE"),
            OpCode::CODECOPY => write!(f, "CODECOPY"),
            OpCode::GASPRICE => write!(f, "GASPRICE"),
            OpCode::EXTCODESIZE => write!(f, "EXTCODESIZE"),
            OpCode::EXTCODECOPY => write!(f, "EXTCODECOPY"),
            OpCode::RETURNDATASIZE => write!(f, "RETURNDATASIZE"),
            OpCode::RETURNDATACOPY => write!(f, "RETURNDATACOPY"),
            OpCode::EXTCODEHASH => write!(f, "EXTCODEHASH"),
            OpCode::BLOCKHASH => write!(f, "BLOCKHASH"),
            OpCode::COINBASE => write!(f, "COINBASE"),
            OpCode::TIMESTAMP => write!(f, "TIMESTAMP"),
            OpCode::NUMBER => write!(f, "NUMBER"),
            OpCode::DIFFICULTY => write!(f, "DIFFICULTY"),
            OpCode::GASLIMIT => write!(f, "GASLIMIT"),
            OpCode::CHAINID => write!(f, "CHAINID"),
            OpCode::SELFBALANCE => write!(f, "SELFBALANCE"),
            OpCode::BASEFEE => write!(f, "BASEFEE"),
            OpCode::POP => write!(f, "POP"),
            OpCode::MLOAD => write!(f, "MLOAD"),
            OpCode::MSTORE => write!(f, "MSTORE"),
            OpCode::MSTORE8 => write!(f, "MSTORE8"),
            OpCode::SLOAD => write!(f, "SLOAD"),
            OpCode::SSTORE => write!(f, "SSTORE"),
            OpCode::JUMP => write!(f, "JUMP"),
            OpCode::JUMPI => write!(f, "JUMPI"),
            OpCode::PC => write!(f, "PC"),
            OpCode::MSIZE => write!(f, "MSIZE"),
            OpCode::GAS => write!(f, "GAS"),
            OpCode::JUMPDEST => write!(f, "JUMPDEST"),
            OpCode::PUSH1 => write!(f, "PUSH1"),
            OpCode::PUSH2 => write!(f, "PUSH2"),
            OpCode::PUSH3 => write!(f, "PUSH3"),
            OpCode::PUSH4 => write!(f, "PUSH4"),
            OpCode::PUSH5 => write!(f, "PUSH5"),
            OpCode::PUSH6 => write!(f, "PUSH6"),
            OpCode::PUSH7 => write!(f, "PUSH7"),
            OpCode::PUSH8 => write!(f, "PUSH8"),
            OpCode::PUSH9 => write!(f, "PUSH9"),
            OpCode::PUSH10 => write!(f, "PUSH10"),
            OpCode::PUSH11 => write!(f, "PUSH11"),
            OpCode::PUSH12 => write!(f, "PUSH12"),
            OpCode::PUSH13 => write!(f, "PUSH13"),
            OpCode::PUSH14 => write!(f, "PUSH14"),
            OpCode::PUSH15 => write!(f, "PUSH15"),
            OpCode::PUSH16 => write!(f, "PUSH16"),
            OpCode::PUSH17 => write!(f, "PUSH17"),
            OpCode::PUSH18 => write!(f, "PUSH18"),
            OpCode::PUSH19 => write!(f, "PUSH19"),
            OpCode::PUSH20 => write!(f, "PUSH20"),
            OpCode::PUSH21 => write!(f, "PUSH21"),
            OpCode::PUSH22 => write!(f, "PUSH22"),
            OpCode::PUSH23 => write!(f, "PUSH23"),
            OpCode::PUSH24 => write!(f, "PUSH24"),
            OpCode::PUSH25 => write!(f, "PUSH25"),
            OpCode::PUSH26 => write!(f, "PUSH26"),
            OpCode::PUSH27 => write!(f, "PUSH27"),
            OpCode::PUSH28 => write!(f, "PUSH28"),
            OpCode::PUSH29 => write!(f, "PUSH29"),
            OpCode::PUSH30 => write!(f, "PUSH30"),
            OpCode::PUSH31 => write!(f, "PUSH31"),
            OpCode::PUSH32 => write!(f, "PUSH32"),
            OpCode::DUP1 => write!(f, "DUP1"),
            OpCode::DUP2 => write!(f, "DUP2"),
            OpCode::DUP3 => write!(f, "DUP3"),
            OpCode::DUP4 => write!(f, "DUP4"),
            OpCode::DUP5 => write!(f, "DUP5"),
            OpCode::DUP6 => write!(f, "DUP6"),
            OpCode::DUP7 => write!(f, "DUP7"),
            OpCode::DUP8 => write!(f, "DUP8"),
            OpCode::DUP9 => write!(f, "DUP9"),
            OpCode::DUP10 => write!(f, "DUP10"),
            OpCode::DUP11 => write!(f, "DUP11"),
            OpCode::DUP12 => write!(f, "DUP12"),
            OpCode::DUP13 => write!(f, "DUP13"),
            OpCode::DUP14 => write!(f, "DUP14"),
            OpCode::DUP15 => write!(f, "DUP15"),
            OpCode::DUP16 => write!(f, "DUP16"),
            OpCode::SWAP1 => write!(f, "SWAP1"),
            OpCode::SWAP2 => write!(f, "SWAP2"),
            OpCode::SWAP3 => write!(f, "SWAP3"),
            OpCode::SWAP4 => write!(f, "SWAP4"),
            OpCode::SWAP5 => write!(f, "SWAP5"),
            OpCode::SWAP6 => write!(f, "SWAP6"),
            OpCode::SWAP7 => write!(f, "SWAP7"),
            OpCode::SWAP8 => write!(f, "SWAP8"),
            OpCode::SWAP9 => write!(f, "SWAP9"),
            OpCode::SWAP10 => write!(f, "SWAP10"),
            OpCode::SWAP11 => write!(f, "SWAP11"),
            OpCode::SWAP12 => write!(f, "SWAP12"),
            OpCode::SWAP13 => write!(f, "SWAP13"),
            OpCode::SWAP14 => write!(f, "SWAP14"),
            OpCode::SWAP15 => write!(f, "SWAP15"),
            OpCode::SWAP16 => write!(f, "SWAP16"),
            OpCode::LOG0 => write!(f, "LOG0"),
            OpCode::LOG1 => write!(f, "LOG1"),
            OpCode::LOG2 => write!(f, "LOG2"),
            OpCode::LOG3 => write!(f, "LOG3"),
            OpCode::LOG4 => write!(f, "LOG4"),
            OpCode::CREATE => write!(f, "CREATE"),
            OpCode::CALL => write!(f, "CALL"),
            OpCode::CALLCODE => write!(f, "CALLCODE"),
            OpCode::RETURN => write!(f, "RETURN"),
            OpCode::DELEGATECALL => write!(f, "DELEGATECALL"),
            OpCode::CREATE2 => write!(f, "CREATE2"),
            OpCode::REVERT => write!(f, "REVERT"),
            OpCode::STATICCALL => write!(f, "STATICCALL"),
            OpCode::SELFDESTRUCT => write!(f, "SELFDESTRUCT"),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum GasPriceTier {
    Zero,
    Base,
    VeryLow,
    Low,
    Mid,
    High,
    Ext,
    Special,
}

impl GasPriceTier {
    pub fn idx(self) -> usize {
        match self {
            GasPriceTier::Zero => 0,
            GasPriceTier::Base => 1,
            GasPriceTier::VeryLow => 2,
            GasPriceTier::Low => 3,
            GasPriceTier::Mid => 4,
            GasPriceTier::High => 5,
            GasPriceTier::Ext => 6,
            GasPriceTier::Special => 7,
        }
    }
}

impl OpCode {
    pub fn from_u8(n: u8) -> Option<OpCode> {
        match n {
            0x00 => Some(OpCode::STOP),
            0x01 => Some(OpCode::ADD),
            0x02 => Some(OpCode::MUL),
            0x03 => Some(OpCode::SUB),
            0x04 => Some(OpCode::DIV),
            0x05 => Some(OpCode::SDIV),
            0x06 => Some(OpCode::MOD),
            0x07 => Some(OpCode::SMOD),
            0x08 => Some(OpCode::ADDMOD),
            0x09 => Some(OpCode::MULMOD),
            0x0a => Some(OpCode::EXP),
            0x0b => Some(OpCode::SIGNEXTEND),
            0x10 => Some(OpCode::LT),
            0x11 => Some(OpCode::GT),
            0x12 => Some(OpCode::SLT),
            0x13 => Some(OpCode::SGT),
            0x14 => Some(OpCode::EQ),
            0x15 => Some(OpCode::ISZERO),
            0x16 => Some(OpCode::AND),
            0x17 => Some(OpCode::OR),
            0x18 => Some(OpCode::XOR),
            0x19 => Some(OpCode::NOT),
            0x1a => Some(OpCode::BYTE),
            0x1b => Some(OpCode::SHL),
            0x1c => Some(OpCode::SHR),
            0x1d => Some(OpCode::SAR),
            0x20 => Some(OpCode::SHA3),
            0x30 => Some(OpCode::ADDRESS),
            0x31 => Some(OpCode::BALANCE),
            0x32 => Some(OpCode::ORIGIN),
            0x33 => Some(OpCode::CALLER),
            0x34 => Some(OpCode::CALLVALUE),
            0x35 => Some(OpCode::CALLDATALOAD),
            0x36 => Some(OpCode::CALLDATASIZE),
            0x37 => Some(OpCode::CALLDATACOPY),
            0x38 => Some(OpCode::CODESIZE),
            0x39 => Some(OpCode::CODECOPY),
            0x3a => Some(OpCode::GASPRICE),
            0x3b => Some(OpCode::EXTCODESIZE),
            0x3c => Some(OpCode::EXTCODECOPY),
            0x3d => Some(OpCode::RETURNDATASIZE),
            0x3e => Some(OpCode::RETURNDATACOPY),
            0x3f => Some(OpCode::EXTCODEHASH),
            0x40 => Some(OpCode::BLOCKHASH),
            0x41 => Some(OpCode::COINBASE),
            0x42 => Some(OpCode::TIMESTAMP),
            0x43 => Some(OpCode::NUMBER),
            0x44 => Some(OpCode::DIFFICULTY),
            0x45 => Some(OpCode::GASLIMIT),
            0x46 => Some(OpCode::CHAINID),
            0x47 => Some(OpCode::SELFBALANCE),
            0x48 => Some(OpCode::BASEFEE),
            0x50 => Some(OpCode::POP),
            0x51 => Some(OpCode::MLOAD),
            0x52 => Some(OpCode::MSTORE),
            0x53 => Some(OpCode::MSTORE8),
            0x54 => Some(OpCode::SLOAD),
            0x55 => Some(OpCode::SSTORE),
            0x56 => Some(OpCode::JUMP),
            0x57 => Some(OpCode::JUMPI),
            0x58 => Some(OpCode::PC),
            0x59 => Some(OpCode::MSIZE),
            0x5a => Some(OpCode::GAS),
            0x5b => Some(OpCode::JUMPDEST),
            0x60 => Some(OpCode::PUSH1),
            0x61 => Some(OpCode::PUSH2),
            0x62 => Some(OpCode::PUSH3),
            0x63 => Some(OpCode::PUSH4),
            0x64 => Some(OpCode::PUSH5),
            0x65 => Some(OpCode::PUSH6),
            0x66 => Some(OpCode::PUSH7),
            0x67 => Some(OpCode::PUSH8),
            0x68 => Some(OpCode::PUSH9),
            0x69 => Some(OpCode::PUSH10),
            0x6a => Some(OpCode::PUSH11),
            0x6b => Some(OpCode::PUSH12),
            0x6c => Some(OpCode::PUSH13),
            0x6d => Some(OpCode::PUSH14),
            0x6e => Some(OpCode::PUSH15),
            0x6f => Some(OpCode::PUSH16),
            0x70 => Some(OpCode::PUSH17),
            0x71 => Some(OpCode::PUSH18),
            0x72 => Some(OpCode::PUSH19),
            0x73 => Some(OpCode::PUSH20),
            0x74 => Some(OpCode::PUSH21),
            0x75 => Some(OpCode::PUSH22),
            0x76 => Some(OpCode::PUSH23),
            0x77 => Some(OpCode::PUSH24),
            0x78 => Some(OpCode::PUSH25),
            0x79 => Some(OpCode::PUSH26),
            0x7a => Some(OpCode::PUSH27),
            0x7b => Some(OpCode::PUSH28),
            0x7c => Some(OpCode::PUSH29),
            0x7d => Some(OpCode::PUSH30),
            0x7e => Some(OpCode::PUSH31),
            0x7f => Some(OpCode::PUSH32),
            0x80 => Some(OpCode::DUP1),
            0x81 => Some(OpCode::DUP2),
            0x82 => Some(OpCode::DUP3),
            0x83 => Some(OpCode::DUP4),
            0x84 => Some(OpCode::DUP5),
            0x85 => Some(OpCode::DUP6),
            0x86 => Some(OpCode::DUP7),
            0x87 => Some(OpCode::DUP8),
            0x88 => Some(OpCode::DUP9),
            0x89 => Some(OpCode::DUP10),
            0x8a => Some(OpCode::DUP11),
            0x8b => Some(OpCode::DUP12),
            0x8c => Some(OpCode::DUP13),
            0x8d => Some(OpCode::DUP14),
            0x8e => Some(OpCode::DUP15),
            0x8f => Some(OpCode::DUP16),
            0x90 => Some(OpCode::SWAP1),
            0x91 => Some(OpCode::SWAP2),
            0x92 => Some(OpCode::SWAP3),
            0x93 => Some(OpCode::SWAP4),
            0x94 => Some(OpCode::SWAP5),
            0x95 => Some(OpCode::SWAP6),
            0x96 => Some(OpCode::SWAP7),
            0x97 => Some(OpCode::SWAP8),
            0x98 => Some(OpCode::SWAP9),
            0x99 => Some(OpCode::SWAP10),
            0x9a => Some(OpCode::SWAP11),
            0x9b => Some(OpCode::SWAP12),
            0x9c => Some(OpCode::SWAP13),
            0x9d => Some(OpCode::SWAP14),
            0x9e => Some(OpCode::SWAP15),
            0x9f => Some(OpCode::SWAP16),
            0xa0 => Some(OpCode::LOG0),
            0xa1 => Some(OpCode::LOG1),
            0xa2 => Some(OpCode::LOG2),
            0xa3 => Some(OpCode::LOG3),
            0xa4 => Some(OpCode::LOG4),
            0xf0 => Some(OpCode::CREATE),
            0xf1 => Some(OpCode::CALL),
            0xf2 => Some(OpCode::CALLCODE),
            0xf3 => Some(OpCode::RETURN),
            0xf4 => Some(OpCode::DELEGATECALL),
            0xf5 => Some(OpCode::CREATE2),
            0xfd => Some(OpCode::REVERT),
            0xfa => Some(OpCode::STATICCALL),
            0xff => Some(OpCode::SELFDESTRUCT),
            _ => None,
        }
    }

    pub fn gas_price_tier(&self) -> GasPriceTier {
        match self {
            OpCode::STOP => GasPriceTier::Zero,
            OpCode::ADD => GasPriceTier::VeryLow,
            OpCode::MUL => GasPriceTier::Low,
            OpCode::SUB => GasPriceTier::VeryLow,
            OpCode::DIV => GasPriceTier::Low,
            OpCode::SDIV => GasPriceTier::Low,
            OpCode::MOD => GasPriceTier::Low,
            OpCode::SMOD => GasPriceTier::Low,
            OpCode::EXP => GasPriceTier::Special,
            OpCode::NOT => GasPriceTier::VeryLow,
            OpCode::LT => GasPriceTier::VeryLow,
            OpCode::GT => GasPriceTier::VeryLow,
            OpCode::SLT => GasPriceTier::VeryLow,
            OpCode::SGT => GasPriceTier::VeryLow,
            OpCode::EQ => GasPriceTier::VeryLow,
            OpCode::ISZERO => GasPriceTier::VeryLow,
            OpCode::AND => GasPriceTier::VeryLow,
            OpCode::OR => GasPriceTier::VeryLow,
            OpCode::XOR => GasPriceTier::VeryLow,
            OpCode::BYTE => GasPriceTier::VeryLow,
            OpCode::SHL => GasPriceTier::VeryLow,
            OpCode::SHR => GasPriceTier::VeryLow,
            OpCode::SAR => GasPriceTier::VeryLow,
            OpCode::ADDMOD => GasPriceTier::Mid,
            OpCode::MULMOD => GasPriceTier::Mid,
            OpCode::SIGNEXTEND => GasPriceTier::Low,
            OpCode::RETURNDATASIZE => GasPriceTier::Base,
            OpCode::RETURNDATACOPY => GasPriceTier::VeryLow,
            OpCode::SHA3 => GasPriceTier::Special,
            OpCode::ADDRESS => GasPriceTier::Base,
            OpCode::BALANCE => GasPriceTier::Special,
            OpCode::ORIGIN => GasPriceTier::Base,
            OpCode::CALLER => GasPriceTier::Base,
            OpCode::CALLVALUE => GasPriceTier::Base,
            OpCode::CALLDATALOAD => GasPriceTier::VeryLow,
            OpCode::CALLDATASIZE => GasPriceTier::Base,
            OpCode::CALLDATACOPY => GasPriceTier::VeryLow,
            OpCode::EXTCODEHASH => GasPriceTier::Special,
            OpCode::CODESIZE => GasPriceTier::Base,
            OpCode::CODECOPY => GasPriceTier::VeryLow,
            OpCode::GASPRICE => GasPriceTier::Base,
            OpCode::EXTCODESIZE => GasPriceTier::Special,
            OpCode::EXTCODECOPY => GasPriceTier::Special,
            OpCode::BLOCKHASH => GasPriceTier::Ext,
            OpCode::COINBASE => GasPriceTier::Base,
            OpCode::TIMESTAMP => GasPriceTier::Base,
            OpCode::NUMBER => GasPriceTier::Base,
            OpCode::DIFFICULTY => GasPriceTier::Base,
            OpCode::GASLIMIT => GasPriceTier::Base,
            OpCode::CHAINID => GasPriceTier::VeryLow,
            OpCode::SELFBALANCE => GasPriceTier::Low,
            OpCode::BASEFEE => GasPriceTier::Base,
            OpCode::POP => GasPriceTier::Base,
            OpCode::MLOAD => GasPriceTier::VeryLow,
            OpCode::MSTORE => GasPriceTier::VeryLow,
            OpCode::MSTORE8 => GasPriceTier::VeryLow,
            OpCode::SLOAD => GasPriceTier::Special,
            OpCode::SSTORE => GasPriceTier::Special,
            OpCode::JUMP => GasPriceTier::Mid,
            OpCode::JUMPI => GasPriceTier::High,
            OpCode::PC => GasPriceTier::Base,
            OpCode::MSIZE => GasPriceTier::Base,
            OpCode::GAS => GasPriceTier::Base,
            OpCode::JUMPDEST => GasPriceTier::Special,
            OpCode::PUSH1 => GasPriceTier::VeryLow,
            OpCode::PUSH2 => GasPriceTier::VeryLow,
            OpCode::PUSH3 => GasPriceTier::VeryLow,
            OpCode::PUSH4 => GasPriceTier::VeryLow,
            OpCode::PUSH5 => GasPriceTier::VeryLow,
            OpCode::PUSH6 => GasPriceTier::VeryLow,
            OpCode::PUSH7 => GasPriceTier::VeryLow,
            OpCode::PUSH8 => GasPriceTier::VeryLow,
            OpCode::PUSH9 => GasPriceTier::VeryLow,
            OpCode::PUSH10 => GasPriceTier::VeryLow,
            OpCode::PUSH11 => GasPriceTier::VeryLow,
            OpCode::PUSH12 => GasPriceTier::VeryLow,
            OpCode::PUSH13 => GasPriceTier::VeryLow,
            OpCode::PUSH14 => GasPriceTier::VeryLow,
            OpCode::PUSH15 => GasPriceTier::VeryLow,
            OpCode::PUSH16 => GasPriceTier::VeryLow,
            OpCode::PUSH17 => GasPriceTier::VeryLow,
            OpCode::PUSH18 => GasPriceTier::VeryLow,
            OpCode::PUSH19 => GasPriceTier::VeryLow,
            OpCode::PUSH20 => GasPriceTier::VeryLow,
            OpCode::PUSH21 => GasPriceTier::VeryLow,
            OpCode::PUSH22 => GasPriceTier::VeryLow,
            OpCode::PUSH23 => GasPriceTier::VeryLow,
            OpCode::PUSH24 => GasPriceTier::VeryLow,
            OpCode::PUSH25 => GasPriceTier::VeryLow,
            OpCode::PUSH26 => GasPriceTier::VeryLow,
            OpCode::PUSH27 => GasPriceTier::VeryLow,
            OpCode::PUSH28 => GasPriceTier::VeryLow,
            OpCode::PUSH29 => GasPriceTier::VeryLow,
            OpCode::PUSH30 => GasPriceTier::VeryLow,
            OpCode::PUSH31 => GasPriceTier::VeryLow,
            OpCode::PUSH32 => GasPriceTier::VeryLow,
            OpCode::DUP1 => GasPriceTier::VeryLow,
            OpCode::DUP2 => GasPriceTier::VeryLow,
            OpCode::DUP3 => GasPriceTier::VeryLow,
            OpCode::DUP4 => GasPriceTier::VeryLow,
            OpCode::DUP5 => GasPriceTier::VeryLow,
            OpCode::DUP6 => GasPriceTier::VeryLow,
            OpCode::DUP7 => GasPriceTier::VeryLow,
            OpCode::DUP8 => GasPriceTier::VeryLow,
            OpCode::DUP9 => GasPriceTier::VeryLow,
            OpCode::DUP10 => GasPriceTier::VeryLow,
            OpCode::DUP11 => GasPriceTier::VeryLow,
            OpCode::DUP12 => GasPriceTier::VeryLow,
            OpCode::DUP13 => GasPriceTier::VeryLow,
            OpCode::DUP14 => GasPriceTier::VeryLow,
            OpCode::DUP15 => GasPriceTier::VeryLow,
            OpCode::DUP16 => GasPriceTier::VeryLow,
            OpCode::SWAP1 => GasPriceTier::VeryLow,
            OpCode::SWAP2 => GasPriceTier::VeryLow,
            OpCode::SWAP3 => GasPriceTier::VeryLow,
            OpCode::SWAP4 => GasPriceTier::VeryLow,
            OpCode::SWAP5 => GasPriceTier::VeryLow,
            OpCode::SWAP6 => GasPriceTier::VeryLow,
            OpCode::SWAP7 => GasPriceTier::VeryLow,
            OpCode::SWAP8 => GasPriceTier::VeryLow,
            OpCode::SWAP9 => GasPriceTier::VeryLow,
            OpCode::SWAP10 => GasPriceTier::VeryLow,
            OpCode::SWAP11 => GasPriceTier::VeryLow,
            OpCode::SWAP12 => GasPriceTier::VeryLow,
            OpCode::SWAP13 => GasPriceTier::VeryLow,
            OpCode::SWAP14 => GasPriceTier::VeryLow,
            OpCode::SWAP15 => GasPriceTier::VeryLow,
            OpCode::SWAP16 => GasPriceTier::VeryLow,
            OpCode::LOG0 => GasPriceTier::Special,
            OpCode::LOG1 => GasPriceTier::Special,
            OpCode::LOG2 => GasPriceTier::Special,
            OpCode::LOG3 => GasPriceTier::Special,
            OpCode::LOG4 => GasPriceTier::Special,
            OpCode::CREATE => GasPriceTier::Special,
            OpCode::CALL => GasPriceTier::Special,
            OpCode::CALLCODE => GasPriceTier::Special,
            OpCode::RETURN => GasPriceTier::Zero,
            OpCode::DELEGATECALL => GasPriceTier::Special,
            OpCode::STATICCALL => GasPriceTier::Special,
            OpCode::SELFDESTRUCT => GasPriceTier::Special,
            OpCode::CREATE2 => GasPriceTier::Special,
            OpCode::REVERT => GasPriceTier::Zero,
        }
    }

    pub fn stack_require(&self) -> u64 {
        match self {
            OpCode::STOP => 0,
            OpCode::ADD => 2,
            OpCode::MUL => 2,
            OpCode::SUB => 2,
            OpCode::DIV => 2,
            OpCode::SDIV => 2,
            OpCode::MOD => 2,
            OpCode::SMOD => 2,
            OpCode::EXP => 2,
            OpCode::NOT => 1,
            OpCode::LT => 2,
            OpCode::GT => 2,
            OpCode::SLT => 2,
            OpCode::SGT => 2,
            OpCode::EQ => 2,
            OpCode::ISZERO => 1,
            OpCode::AND => 2,
            OpCode::OR => 2,
            OpCode::XOR => 2,
            OpCode::BYTE => 2,
            OpCode::SHL => 2,
            OpCode::SHR => 2,
            OpCode::SAR => 2,
            OpCode::ADDMOD => 3,
            OpCode::MULMOD => 3,
            OpCode::SIGNEXTEND => 2,
            OpCode::RETURNDATASIZE => 0,
            OpCode::RETURNDATACOPY => 3,
            OpCode::SHA3 => 2,
            OpCode::ADDRESS => 0,
            OpCode::BALANCE => 1,
            OpCode::ORIGIN => 0,
            OpCode::CALLER => 0,
            OpCode::CALLVALUE => 0,
            OpCode::CALLDATALOAD => 1,
            OpCode::CALLDATASIZE => 0,
            OpCode::CALLDATACOPY => 3,
            OpCode::EXTCODEHASH => 1,
            OpCode::CODESIZE => 0,
            OpCode::CODECOPY => 3,
            OpCode::GASPRICE => 0,
            OpCode::EXTCODESIZE => 1,
            OpCode::EXTCODECOPY => 4,
            OpCode::BLOCKHASH => 1,
            OpCode::COINBASE => 0,
            OpCode::TIMESTAMP => 0,
            OpCode::NUMBER => 0,
            OpCode::DIFFICULTY => 0,
            OpCode::GASLIMIT => 0,
            OpCode::CHAINID => 0,
            OpCode::SELFBALANCE => 0,
            OpCode::BASEFEE => 0,
            OpCode::POP => 1,
            OpCode::MLOAD => 1,
            OpCode::MSTORE => 2,
            OpCode::MSTORE8 => 2,
            OpCode::SLOAD => 1,
            OpCode::SSTORE => 2,
            OpCode::JUMP => 1,
            OpCode::JUMPI => 2,
            OpCode::PC => 0,
            OpCode::MSIZE => 0,
            OpCode::GAS => 0,
            OpCode::JUMPDEST => 0,
            OpCode::PUSH1 => 0,
            OpCode::PUSH2 => 0,
            OpCode::PUSH3 => 0,
            OpCode::PUSH4 => 0,
            OpCode::PUSH5 => 0,
            OpCode::PUSH6 => 0,
            OpCode::PUSH7 => 0,
            OpCode::PUSH8 => 0,
            OpCode::PUSH9 => 0,
            OpCode::PUSH10 => 0,
            OpCode::PUSH11 => 0,
            OpCode::PUSH12 => 0,
            OpCode::PUSH13 => 0,
            OpCode::PUSH14 => 0,
            OpCode::PUSH15 => 0,
            OpCode::PUSH16 => 0,
            OpCode::PUSH17 => 0,
            OpCode::PUSH18 => 0,
            OpCode::PUSH19 => 0,
            OpCode::PUSH20 => 0,
            OpCode::PUSH21 => 0,
            OpCode::PUSH22 => 0,
            OpCode::PUSH23 => 0,
            OpCode::PUSH24 => 0,
            OpCode::PUSH25 => 0,
            OpCode::PUSH26 => 0,
            OpCode::PUSH27 => 0,
            OpCode::PUSH28 => 0,
            OpCode::PUSH29 => 0,
            OpCode::PUSH30 => 0,
            OpCode::PUSH31 => 0,
            OpCode::PUSH32 => 0,
            OpCode::DUP1 => 1,
            OpCode::DUP2 => 2,
            OpCode::DUP3 => 3,
            OpCode::DUP4 => 4,
            OpCode::DUP5 => 5,
            OpCode::DUP6 => 6,
            OpCode::DUP7 => 7,
            OpCode::DUP8 => 8,
            OpCode::DUP9 => 9,
            OpCode::DUP10 => 10,
            OpCode::DUP11 => 11,
            OpCode::DUP12 => 12,
            OpCode::DUP13 => 13,
            OpCode::DUP14 => 14,
            OpCode::DUP15 => 15,
            OpCode::DUP16 => 16,
            OpCode::SWAP1 => 2,
            OpCode::SWAP2 => 3,
            OpCode::SWAP3 => 4,
            OpCode::SWAP4 => 5,
            OpCode::SWAP5 => 6,
            OpCode::SWAP6 => 7,
            OpCode::SWAP7 => 8,
            OpCode::SWAP8 => 9,
            OpCode::SWAP9 => 10,
            OpCode::SWAP10 => 11,
            OpCode::SWAP11 => 12,
            OpCode::SWAP12 => 13,
            OpCode::SWAP13 => 14,
            OpCode::SWAP14 => 15,
            OpCode::SWAP15 => 16,
            OpCode::SWAP16 => 17,
            OpCode::LOG0 => 2,
            OpCode::LOG1 => 3,
            OpCode::LOG2 => 4,
            OpCode::LOG3 => 5,
            OpCode::LOG4 => 6,
            OpCode::CREATE => 3,
            OpCode::CALL => 7,
            OpCode::CALLCODE => 7,
            OpCode::RETURN => 2,
            OpCode::DELEGATECALL => 6,
            OpCode::STATICCALL => 6,
            OpCode::SELFDESTRUCT => 1,
            OpCode::CREATE2 => 4,
            OpCode::REVERT => 2,
        }
    }

    pub fn stack_returns(&self) -> u64 {
        match self {
            OpCode::STOP => 0,
            OpCode::ADD => 1,
            OpCode::MUL => 1,
            OpCode::SUB => 1,
            OpCode::DIV => 1,
            OpCode::SDIV => 1,
            OpCode::MOD => 1,
            OpCode::SMOD => 1,
            OpCode::EXP => 1,
            OpCode::NOT => 1,
            OpCode::LT => 1,
            OpCode::GT => 1,
            OpCode::SLT => 1,
            OpCode::SGT => 1,
            OpCode::EQ => 1,
            OpCode::ISZERO => 1,
            OpCode::AND => 1,
            OpCode::OR => 1,
            OpCode::XOR => 1,
            OpCode::BYTE => 1,
            OpCode::SHL => 1,
            OpCode::SHR => 1,
            OpCode::SAR => 1,
            OpCode::ADDMOD => 1,
            OpCode::MULMOD => 1,
            OpCode::SIGNEXTEND => 1,
            OpCode::RETURNDATASIZE => 1,
            OpCode::RETURNDATACOPY => 0,
            OpCode::SHA3 => 1,
            OpCode::ADDRESS => 1,
            OpCode::BALANCE => 1,
            OpCode::ORIGIN => 1,
            OpCode::CALLER => 1,
            OpCode::CALLVALUE => 1,
            OpCode::CALLDATALOAD => 1,
            OpCode::CALLDATASIZE => 1,
            OpCode::CALLDATACOPY => 0,
            OpCode::EXTCODEHASH => 1,
            OpCode::CODESIZE => 1,
            OpCode::CODECOPY => 0,
            OpCode::GASPRICE => 1,
            OpCode::EXTCODESIZE => 1,
            OpCode::EXTCODECOPY => 0,
            OpCode::BLOCKHASH => 1,
            OpCode::COINBASE => 1,
            OpCode::TIMESTAMP => 1,
            OpCode::NUMBER => 1,
            OpCode::DIFFICULTY => 1,
            OpCode::GASLIMIT => 1,
            OpCode::CHAINID => 1,
            OpCode::SELFBALANCE => 1,
            OpCode::BASEFEE => 1,
            OpCode::POP => 0,
            OpCode::MLOAD => 1,
            OpCode::MSTORE => 0,
            OpCode::MSTORE8 => 0,
            OpCode::SLOAD => 1,
            OpCode::SSTORE => 0,
            OpCode::JUMP => 0,
            OpCode::JUMPI => 0,
            OpCode::PC => 1,
            OpCode::MSIZE => 1,
            OpCode::GAS => 1,
            OpCode::JUMPDEST => 0,
            OpCode::PUSH1 => 1,
            OpCode::PUSH2 => 1,
            OpCode::PUSH3 => 1,
            OpCode::PUSH4 => 1,
            OpCode::PUSH5 => 1,
            OpCode::PUSH6 => 1,
            OpCode::PUSH7 => 1,
            OpCode::PUSH8 => 1,
            OpCode::PUSH9 => 1,
            OpCode::PUSH10 => 1,
            OpCode::PUSH11 => 1,
            OpCode::PUSH12 => 1,
            OpCode::PUSH13 => 1,
            OpCode::PUSH14 => 1,
            OpCode::PUSH15 => 1,
            OpCode::PUSH16 => 1,
            OpCode::PUSH17 => 1,
            OpCode::PUSH18 => 1,
            OpCode::PUSH19 => 1,
            OpCode::PUSH20 => 1,
            OpCode::PUSH21 => 1,
            OpCode::PUSH22 => 1,
            OpCode::PUSH23 => 1,
            OpCode::PUSH24 => 1,
            OpCode::PUSH25 => 1,
            OpCode::PUSH26 => 1,
            OpCode::PUSH27 => 1,
            OpCode::PUSH28 => 1,
            OpCode::PUSH29 => 1,
            OpCode::PUSH30 => 1,
            OpCode::PUSH31 => 1,
            OpCode::PUSH32 => 1,
            OpCode::DUP1 => 2,
            OpCode::DUP2 => 3,
            OpCode::DUP3 => 4,
            OpCode::DUP4 => 5,
            OpCode::DUP5 => 6,
            OpCode::DUP6 => 7,
            OpCode::DUP7 => 8,
            OpCode::DUP8 => 9,
            OpCode::DUP9 => 10,
            OpCode::DUP10 => 11,
            OpCode::DUP11 => 12,
            OpCode::DUP12 => 13,
            OpCode::DUP13 => 14,
            OpCode::DUP14 => 15,
            OpCode::DUP15 => 16,
            OpCode::DUP16 => 17,
            OpCode::SWAP1 => 2,
            OpCode::SWAP2 => 3,
            OpCode::SWAP3 => 4,
            OpCode::SWAP4 => 5,
            OpCode::SWAP5 => 6,
            OpCode::SWAP6 => 7,
            OpCode::SWAP7 => 8,
            OpCode::SWAP8 => 9,
            OpCode::SWAP9 => 10,
            OpCode::SWAP10 => 11,
            OpCode::SWAP11 => 12,
            OpCode::SWAP12 => 13,
            OpCode::SWAP13 => 14,
            OpCode::SWAP14 => 15,
            OpCode::SWAP15 => 16,
            OpCode::SWAP16 => 17,
            OpCode::LOG0 => 0,
            OpCode::LOG1 => 0,
            OpCode::LOG2 => 0,
            OpCode::LOG3 => 0,
            OpCode::LOG4 => 0,
            OpCode::CREATE => 1,
            OpCode::CALL => 1,
            OpCode::CALLCODE => 1,
            OpCode::RETURN => 0,
            OpCode::DELEGATECALL => 1,
            OpCode::STATICCALL => 1,
            OpCode::SELFDESTRUCT => 0,
            OpCode::CREATE2 => 1,
            OpCode::REVERT => 0,
        }
    }

    pub fn state_changes(&self) -> bool {
        match self {
            OpCode::STOP => false,
            OpCode::ADD => false,
            OpCode::SUB => false,
            OpCode::MUL => false,
            OpCode::DIV => false,
            OpCode::SDIV => false,
            OpCode::MOD => false,
            OpCode::SMOD => false,
            OpCode::EXP => false,
            OpCode::NOT => false,
            OpCode::LT => false,
            OpCode::GT => false,
            OpCode::SLT => false,
            OpCode::SGT => false,
            OpCode::EQ => false,
            OpCode::ISZERO => false,
            OpCode::AND => false,
            OpCode::OR => false,
            OpCode::XOR => false,
            OpCode::BYTE => false,
            OpCode::SHL => false,
            OpCode::SHR => false,
            OpCode::SAR => false,
            OpCode::ADDMOD => false,
            OpCode::MULMOD => false,
            OpCode::SIGNEXTEND => false,
            OpCode::RETURNDATASIZE => false,
            OpCode::RETURNDATACOPY => false,
            OpCode::SHA3 => false,
            OpCode::ADDRESS => false,
            OpCode::BALANCE => false,
            OpCode::ORIGIN => false,
            OpCode::CALLER => false,
            OpCode::CALLVALUE => false,
            OpCode::CALLDATALOAD => false,
            OpCode::CALLDATASIZE => false,
            OpCode::CALLDATACOPY => false,
            OpCode::EXTCODEHASH => false,
            OpCode::CODESIZE => false,
            OpCode::CODECOPY => false,
            OpCode::GASPRICE => false,
            OpCode::EXTCODESIZE => false,
            OpCode::EXTCODECOPY => false,
            OpCode::BLOCKHASH => false,
            OpCode::COINBASE => false,
            OpCode::TIMESTAMP => false,
            OpCode::NUMBER => false,
            OpCode::DIFFICULTY => false,
            OpCode::GASLIMIT => false,
            OpCode::CHAINID => false,
            OpCode::SELFBALANCE => false,
            OpCode::BASEFEE => false,
            OpCode::POP => false,
            OpCode::MLOAD => false,
            OpCode::MSTORE => false,
            OpCode::MSTORE8 => false,
            OpCode::SLOAD => false,
            OpCode::SSTORE => true,
            OpCode::JUMP => false,
            OpCode::JUMPI => false,
            OpCode::PC => false,
            OpCode::MSIZE => false,
            OpCode::GAS => false,
            OpCode::JUMPDEST => false,
            OpCode::PUSH1 => false,
            OpCode::PUSH2 => false,
            OpCode::PUSH3 => false,
            OpCode::PUSH4 => false,
            OpCode::PUSH5 => false,
            OpCode::PUSH6 => false,
            OpCode::PUSH7 => false,
            OpCode::PUSH8 => false,
            OpCode::PUSH9 => false,
            OpCode::PUSH10 => false,
            OpCode::PUSH11 => false,
            OpCode::PUSH12 => false,
            OpCode::PUSH13 => false,
            OpCode::PUSH14 => false,
            OpCode::PUSH15 => false,
            OpCode::PUSH16 => false,
            OpCode::PUSH17 => false,
            OpCode::PUSH18 => false,
            OpCode::PUSH19 => false,
            OpCode::PUSH20 => false,
            OpCode::PUSH21 => false,
            OpCode::PUSH22 => false,
            OpCode::PUSH23 => false,
            OpCode::PUSH24 => false,
            OpCode::PUSH25 => false,
            OpCode::PUSH26 => false,
            OpCode::PUSH27 => false,
            OpCode::PUSH28 => false,
            OpCode::PUSH29 => false,
            OpCode::PUSH30 => false,
            OpCode::PUSH31 => false,
            OpCode::PUSH32 => false,
            OpCode::DUP1 => false,
            OpCode::DUP2 => false,
            OpCode::DUP3 => false,
            OpCode::DUP4 => false,
            OpCode::DUP5 => false,
            OpCode::DUP6 => false,
            OpCode::DUP7 => false,
            OpCode::DUP8 => false,
            OpCode::DUP9 => false,
            OpCode::DUP10 => false,
            OpCode::DUP11 => false,
            OpCode::DUP12 => false,
            OpCode::DUP13 => false,
            OpCode::DUP14 => false,
            OpCode::DUP15 => false,
            OpCode::DUP16 => false,
            OpCode::SWAP1 => false,
            OpCode::SWAP2 => false,
            OpCode::SWAP3 => false,
            OpCode::SWAP4 => false,
            OpCode::SWAP5 => false,
            OpCode::SWAP6 => false,
            OpCode::SWAP7 => false,
            OpCode::SWAP8 => false,
            OpCode::SWAP9 => false,
            OpCode::SWAP10 => false,
            OpCode::SWAP11 => false,
            OpCode::SWAP12 => false,
            OpCode::SWAP13 => false,
            OpCode::SWAP14 => false,
            OpCode::SWAP15 => false,
            OpCode::SWAP16 => false,
            OpCode::LOG0 => true,
            OpCode::LOG1 => true,
            OpCode::LOG2 => true,
            OpCode::LOG3 => true,
            OpCode::LOG4 => true,
            OpCode::CREATE => true,
            OpCode::CALL => false,
            OpCode::CALLCODE => false,
            OpCode::RETURN => false,
            OpCode::DELEGATECALL => false,
            OpCode::STATICCALL => false,
            OpCode::SELFDESTRUCT => true,
            OpCode::CREATE2 => true,
            OpCode::REVERT => false,
        }
    }
}
