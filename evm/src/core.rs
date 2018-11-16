use super::memory;
use super::opcodes;
use super::stack;
use ethereum_types::*;

pub struct Contract {
    pub code: Vec<u8>,
    pub cgas: u64,
}

impl Contract {
    pub fn new() -> Self {
        Contract {
            code: vec![0; 0],
            cgas: 21000,
        }
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

    pub fn use_gas(&mut self, gas: u64) -> bool {
        if self.cgas < gas {
            return false
        }
        self.cgas -= gas;
        return true
    }
}

pub struct EVMInfo {}

impl EVMInfo {
    pub fn new() -> Self {
        EVMInfo {}
    }
}

pub struct EVMConf {
    pub tier_step_gas: [u64; 8],
}

impl EVMConf {
    pub fn new() -> Self {
        EVMConf {
            tier_step_gas: [0, 2, 3, 5, 8, 10, 20, 0],
        }
    }
}

pub struct EVMContext {
    pub stack: stack::Stack<U256>,
    pub memory: memory::Memory,
    pub info: EVMInfo,
    pub conf: EVMConf,
    pub contract: Contract,
    pub return_data: Vec<u8>,
}

impl EVMContext {
    pub fn new() -> Self {
        EVMContext {
            stack: stack::Stack::with_capacity(1024),
            memory: memory::Memory::new(),
            info: EVMInfo::new(),
            conf: EVMConf::new(),
            contract: Contract::new(),
            return_data: Vec::new(),
        }
    }
}
