extern crate cita_evm;
extern crate ethereum_types;

use cita_evm::evm;
use cita_evm::interpreter::interpreter;
use cita_evm::interpreter::jump_table;
use cita_evm::interpreter::memory;
use cita_evm::interpreter::stack;
use cita_evm::statedb::statedb::{Log, StateDB};
use ethereum_types::{Address, H256, U256};

fn hashfunc(num: U256) -> H256 {
    H256::from(0)
}

struct DB {}

impl StateDB for DB {
    fn create_account(&mut self, address: Address) {}
    fn sub_balance(&mut self, address: Address, value: U256) {}
    fn add_balance(&mut self, address: Address, value: U256) {}
    fn get_balance(&self, address: Address) -> U256 {
        U256::from(0)
    }
    fn get_nonce(&self, address: Address) -> String {
        String::from("0")
    }
    fn set_nonce(&mut self, address: Address, nonce: String) {}
    fn get_code_hash(&self, address: Address) -> H256 {
        H256::from(0)
    }
    fn get_code(&self, address: Address) -> Vec<u8> {
        vec![0; 8]
    }
    fn set_code(&mut self, address: Address, code: &[u8]) {}
    fn get_code_size(&self, address: Address) -> usize {
        0
    }
    fn add_refund(&mut self, quota: u64) {}
    fn set_refund(&mut self, quota: u64) {}
    fn get_refund(&self) -> u64 {
        0
    }
    fn get_committed_state(&self, address: Address, hash: H256) -> H256 {
        H256::from(0)
    }
    fn get_state(&self, address: Address, key: H256) -> H256 {
        H256::from(0)
    }
    fn set_state(&mut self, address: Address, key: H256, value: H256) {}
    fn suicide(&mut self, address: Address) -> bool {
        false
    }
    fn has_suicided(&self, address: Address) -> bool {
        false
    }
    fn exists(&self, address: Address) -> bool {
        false
    }
    fn empty(&self, address: Address) -> bool {
        false
    }
    fn revert_to_snapshot(&mut self, id: usize) {}
    fn snapshot(&mut self) -> usize {
        0
    }
    fn add_log(&mut self, log: Log) {}
    fn add_preimage(&mut self, hash: H256, preimage: &[u8]) {}
}

#[test]
fn test_interpreter_new() {
    let cfg = interpreter::Config {
        jump_table: jump_table::new_instruction_set(),
    };
    let evm_info = evm::EVMInfo {
        origin: Address::zero(),
        block_number: U256::from(0),
        quota_price: U256::from(100000000),
        quota_limit: 210000,
    };
    let ctx = evm::EVMContext {
        stack: stack::Stack::with_capacity(1024),
        memory: memory::Memory::new(),
        state_db: Box::new(DB {}),
        fn_get_hash: hashfunc,
        return_data: vec![0; 8],
        info: evm_info,
        depth: 0,
        abort: false,
    };
    let itp = interpreter::Interpreter::new(ctx, cfg);
}
