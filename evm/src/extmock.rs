use super::err;
use super::ext;
use super::interpreter;
use super::opcodes;
use ethereum_types::*;
use keccak_hash;
use rlp;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Account {
    pub balance: U256,
    pub code: Vec<u8>,
    pub nonce: U256,
    pub storage: BTreeMap<H256, H256>,
}

impl Account {
    pub fn default() -> Self {
        Account {
            balance: U256::zero(),
            code: Vec::new(),
            nonce: U256::zero(),
            storage: BTreeMap::new(),
        }
    }
}

pub struct DataProviderMock {
    pub storage: BTreeMap<Address, Account>,
    pub storage_origin: BTreeMap<String, H256>,
    pub refund: u64,
}

impl ext::DataProvider for DataProviderMock {
    fn get_balance(&self, address: &Address) -> U256 {
        if let Some(data) = self.storage.get(address) {
            return data.balance;
        }
        return U256::zero();
    }

    fn add_refund(&mut self, n: u64) {
        self.refund += n
    }
    fn sub_refund(&mut self, n: u64) {
        self.refund -= n
    }
    fn get_refund(&self) -> u64 {
        self.refund
    }

    fn get_code_size(&self, address: &Address) -> u64 {
        if let Some(data) = self.storage.get(address) {
            return data.code.len() as u64;
        }
        return 0;
    }

    fn get_code(&self, address: &Address) -> &[u8] {
        if let Some(data) = self.storage.get(address) {
            return data.code.as_slice();
        }
        return &[0u8][..];
    }

    fn get_code_hash(&self, address: &Address) -> H256 {
        if let Some(data) = self.storage.get(address) {
            return self.sha3(data.code.as_slice());
        }
        H256::zero()
    }

    fn get_block_hash(&self, _: &U256) -> H256 {
        H256::zero()
    }

    fn get_storage(&self, address: &Address, key: &H256) -> H256 {
        if let Some(data) = self.storage.get(address) {
            if let Some(r) = data.storage.get(key) {
                return *r;
            }
            return H256::zero();
        }
        return H256::zero();
    }

    fn set_storage(&mut self, address: &Address, key: H256, value: H256) {
        if !self.storage.contains_key(address) {
            self.storage.insert(
                *address,
                Account {
                    balance: U256::zero(),
                    code: Vec::new(),
                    nonce: U256::zero(),
                    storage: BTreeMap::new(),
                },
            );
        }
        let account = self.storage.get_mut(address).unwrap();
        account.storage.insert(key, value);
    }

    fn get_storage_origin(&self, address: &Address, key: &H256) -> H256 {
        let fullkey = format!("{}{}", address, key);
        if self.storage_origin.contains_key(&fullkey) {
            return *self.storage_origin.get(&fullkey).unwrap();
        }
        H256::zero()
    }

    fn set_storage_origin(&mut self, address: &Address, key: H256, value: H256) {
        let fullkey = format!("{}{}", address, key);
        self.storage_origin.insert(fullkey, value);
    }

    fn suicide(&mut self, _: &Address) {}

    fn sha3(&self, data: &[u8]) -> H256 {
        keccak_hash::keccak(data)
    }
    fn is_empty(&self, _: &Address) -> bool {
        false
    }

    fn call(
        &self,
        address: &Address,
        input: &[u8],
        gas: u64,
        value: U256,
        op: opcodes::OpCode,
    ) -> (Result<interpreter::InterpreterResult, err::Error>) {
        match op {
            opcodes::OpCode::CREATE => {
                let mut stream = rlp::RlpStream::new_list(2);
                stream.append(&Address::zero()); // Origin address
                stream.append(&U256::zero()); // Nonce of origin address
                let contract_address = Address::from(self.sha3(stream.as_raw()));
                let mut it = interpreter::Interpreter::default();
                it.code_address = contract_address;
                it.code_data = Vec::from(input);
                it.gas = gas;
                it.value = value;
                it.cfg.print_op = true;
                it.cfg.print_stack = true;
                it.cfg.print_gas_used = true;
                it.cfg.print_mem = true;
                println!("[LOG] RUN....");
                let r = it.run()?;
                if let interpreter::InterpreterResult::Normal(ret, gas, _) = r {
                    println!("[LOG] COM....");
                    return Ok(interpreter::InterpreterResult::Create(
                        ret,
                        contract_address,
                        gas,
                    ));
                } else {
                    println!("[LOG] COM....");
                    return Ok(r);
                };
            }
            opcodes::OpCode::CALL => {
                let mut it = interpreter::Interpreter::default();
                it.code_address = *address;
                it.code_data = Vec::from(self.get_code(address));
                it.gas = gas;
                it.value = value;
                let mut data_provider = DataProviderMock::new();
                data_provider.storage = self.storage.clone();
                it.data_provider = Box::new(data_provider);
                it.cfg.print_op = true;
                it.cfg.print_stack = true;
                it.cfg.print_gas_used = true;
                it.cfg.print_mem = true;
                println!("[LOG] RUN....");
                let r = it.run();
                println!("[LOG] COM....");
                return r;
            }
            _ => {}
        }
        Ok(interpreter::InterpreterResult::Normal(
            Vec::new(),
            0,
            Vec::new(),
        ))
    }
}

impl DataProviderMock {
    pub fn new() -> Self {
        DataProviderMock {
            storage: BTreeMap::new(),
            storage_origin: BTreeMap::new(),
            refund: 0,
        }
    }
}
