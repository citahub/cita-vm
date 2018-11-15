use super::memory;
use super::opcodes;
use super::stack;
use ethereum_types::*;

pub struct Contract {
    pub code: Vec<u8>,
}

impl Contract {
    pub fn new() -> Self {
        Contract { code: vec![0; 0] }
    }

    pub fn get_byte(&self, n: u64) -> u8 {
        if n < self.code.len() as u64 {
            return *self.code.get(n as usize).unwrap();
        }
        0
    }

    pub fn get_opcode(&self, n: u64) -> opcodes::OpCode {
        opcodes::OpCode::from(self.get_byte(n))
    }
}

pub struct EVMInfo {}

impl EVMInfo {
    pub fn new() -> Self {
        EVMInfo {}
    }
}

pub struct EVMContext {
    pub stack: stack::Stack<U256>,
    pub memory: memory::Memory,
    pub info: EVMInfo,
    pub contract: Contract,
    pub return_data: Vec<u8>,
}

impl EVMContext {
    pub fn new() -> Self {
        EVMContext {
            stack: stack::Stack::with_capacity(1024),
            memory: memory::Memory::new(),
            info: EVMInfo::new(),
            contract: Contract::new(),
            return_data: Vec::new(),
        }
    }
}
