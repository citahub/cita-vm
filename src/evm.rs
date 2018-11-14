use ethereum_types::{Address, H256, U256};

use super::interpreter::memory::Memory;
use super::interpreter::stack::Stack;
use super::statedb::statedb::StateDB;
use super::opcode::OpCode;
use super::opcode::u8_2_opcode;

pub struct EVMContext {
    pub stack: Stack<U256>,
    pub memory: Memory,
    pub state_db: Box<dyn StateDB>,

    pub fn_get_hash: fn(num: U256) -> H256,

    pub return_data: Vec<u8>,

    pub info: EVMInfo,

    pub depth: u64,
    pub abort: bool,
}

pub struct EVMInfo {
    pub origin: Address,
    pub block_number: U256,
    pub quota_price: U256,
    pub quota_limit: u64,
}

pub struct Contract {
    pub caller_address: Address,
    pub caller: Address,
    pub contract_address: Address,

    pub code: Vec<u8>,
    pub code_hash: H256,
    pub code_address: Address,
    pub input: Vec<u8>,

    pub quota: u64,
    pub value: U256,
}

impl Contract {
    pub fn get_byte(&mut self, n: u64) -> u8 {
        if n < self.code.len() as u64 {
            return *self.code.get(n as usize).unwrap();
        }
        0
    }

    pub fn get_opcode(&mut self, n:u64) -> OpCode {
         u8_2_opcode(self.get_byte(n))
    }

    pub fn use_gas(&mut self, gas: u64) -> bool {
        if self.quota < gas {
            return false
        }
        self.quota -= gas;
        return true
    }
}
