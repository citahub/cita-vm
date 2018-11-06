use ethereum_types::{Address, H256, U256};

use super::interpreter::memory::Memory;
use super::interpreter::stack::Stack;
use super::statedb::statedb::StateDB;

pub struct EVMContext {
    pub stack: Stack<U256>,
    pub memory: Memory,
    pub state_db: Box<dyn StateDB>,

    pub fn_get_hash: fn(num: U256) -> H256,

    pub return_data: Vec<u8>,

    pub info: EVMInfo,
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
