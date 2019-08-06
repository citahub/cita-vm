use ethereum_types::{Address, H256, U256};

use hashbrown::HashMap;

use crate::common::hash::summary;
use crate::evm::err::Error;
use crate::evm::ext::DataProvider;
use crate::evm::interpreter::{Interpreter, InterpreterConf};
use crate::evm::opcodes::OpCode;
use crate::{Context, InterpreterParams, InterpreterResult};

#[derive(Clone, Default)]
pub struct Account {
    pub balance: U256,
    pub code: Vec<u8>,
    pub nonce: U256,
    pub storage: HashMap<H256, H256>,
}

#[derive(Default)]
pub struct DataProviderMock {
    pub db: HashMap<Address, Account>,
    pub db_origin: HashMap<Address, Account>,
    pub refund: HashMap<Address, u64>,
}

impl DataProvider for DataProviderMock {
    fn get_balance(&self, address: &Address) -> U256 {
        self.db.get(address).map_or(U256::zero(), |v| v.balance)
    }

    fn add_refund(&mut self, address: &Address, n: u64) {
        self.refund.entry(*address).and_modify(|v| *v += n).or_insert(n);
    }

    fn sub_refund(&mut self, address: &Address, n: u64) {
        self.refund.entry(*address).and_modify(|v| *v -= n).or_insert(n);
    }

    fn get_refund(&self, address: &Address) -> u64 {
        self.refund.get(address).map_or(0, |v| *v)
    }

    fn get_code_size(&self, address: &Address) -> u64 {
        self.db.get(address).map_or(0, |v| v.code.len() as u64)
    }

    fn get_code(&self, address: &Address) -> Vec<u8> {
        self.db.get(address).map_or(vec![], |v| v.code.clone())
    }

    fn get_code_hash(&self, address: &Address) -> H256 {
        self.db
            .get(address)
            .map_or(H256::zero(), |v| self.sha3(v.code.as_slice()))
    }

    fn get_block_hash(&self, n: &U256) -> H256 {
        H256::from(n)
    }

    fn get_storage(&self, address: &Address, key: &H256) -> H256 {
        self.db
            .get(address)
            .map_or(H256::zero(), |v| v.storage.get(key).map_or(H256::zero(), |&v| v))
    }

    fn set_storage(&mut self, address: &Address, key: H256, value: H256) {
        self.db
            .entry(*address)
            .or_insert_with(Account::default)
            .storage
            .insert(key, value);
    }

    fn get_storage_origin(&self, address: &Address, key: &H256) -> H256 {
        self.db_origin
            .get(address)
            .map_or(H256::zero(), |v| v.storage.get(key).map_or(H256::zero(), |&v| v))
    }

    fn set_storage_origin(&mut self, address: &Address, key: H256, value: H256) {
        self.db_origin
            .entry(*address)
            .or_insert_with(Account::default)
            .storage
            .insert(key, value);
    }

    fn selfdestruct(&mut self, address: &Address, _: &Address) -> bool {
        self.db.remove(address);
        true
    }

    fn sha3(&self, data: &[u8]) -> H256 {
        H256::from(&summary(data)[..])
    }

    fn is_empty(&self, address: &Address) -> bool {
        self.db.get(address).is_none()
    }

    fn call(&self, opcode: OpCode, params: InterpreterParams) -> (Result<InterpreterResult, Error>) {
        match opcode {
            OpCode::CALL => {
                let mut it = Interpreter::new(
                    Context::default(),
                    InterpreterConf::default(),
                    Box::new(DataProviderMock::default()),
                    params,
                );
                let mut data_provider = DataProviderMock::default();
                data_provider.db = self.db.clone();
                it.data_provider = Box::new(data_provider);
                it.run()
            }
            _ => unimplemented!(),
        }
    }
}
