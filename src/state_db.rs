use ethereum_types::{Address, H256, U256};
use evm::ext::DataProvider;
use evm::extmock::DataProviderMock;
use evm::interpreter::{Interpreter, InterpreterConf};
use evm::{err, interpreter, opcodes};
use state::state::State;

pub struct StateDB {
    pub state: State,
}

impl DataProvider for StateDB {
    fn get_balance(&mut self, address: &Address) -> U256 {
        self.state
            .ensure_cached(address)
            .map_or(U256::default(), |account| *account.balance())
    }

    fn get_code(&mut self, address: &Address) -> Vec<u8> {
        if let Some(account) = self.state.ensure_cached(address) {
            if let Some(code) = account.code() {
                return code.clone();
            }
        }
        vec![]
    }

    fn get_code_size(&mut self, address: &Address) -> u64 {
        if let Some(account) = self.state.ensure_cached(address) {
            if let Some(code) = account.code() {
                return code.len() as u64;
            }
        }
        0
    }

    fn get_code_hash(&mut self, address: &Address) -> H256 {
        if let Some(account) = self.state.ensure_cached(address) {
            if let Some(code) = account.code() {
                return self.sha3(&code);
            }
        }
        H256::zero()
    }

    fn add_refund(&mut self, address: &Address, n: u64) {
        self.state.add_refund(address, n);
    }

    fn sub_refund(&mut self, address: &Address, n: u64) {
        self.state.sub_refund(address, n);
    }

    fn get_refund(&self, address: &Address) -> u64 {
        self.state.refund.get(address).map_or(0, |v| *v)
    }

    // TODO
    fn get_block_hash(&self, number: &U256) -> H256 {
        return H256::default();
    }

    fn set_storage(&mut self, address: &Address, key: H256, value: H256) {
        self.state.set_storage(address, key, value);
    }

    fn get_storage(&mut self, address: &Address, key: &H256) -> H256 {
        self.state.storage_at(address, key)
    }

    // TODO:
    fn set_storage_origin(&mut self, address: &Address, key: H256, value: H256) {}

    // TODO
    fn get_storage_origin(&self, address: &Address, key: &H256) -> H256 {
        return H256::default();
    }

    fn selfdestruct(&mut self, address: &Address, refund_address: &Address) {
        self.state.kill_contract(address);
    }

    fn sha3(&self, input: &[u8]) -> H256 {
        keccak_hash::keccak(input)
    }

    // is_empty returns whether the given account is empty. Empty
    // is defined according to EIP161 (balance = nonce = code = 0).
    fn is_empty(&mut self, address: &Address) -> bool {
        self.state.is_empty(address)
    }

    fn call(
        &self,
        opcode: opcodes::OpCode,
        params: interpreter::InterpreterParams,
    ) -> (Result<interpreter::InterpreterResult, err::Error>) {
        let mut it = Interpreter::new(
            self.state.context.clone(),
            InterpreterConf::default(),
            // TODO: cita-tri memoryDB should implements DataProvider trait
            // Box::new(self.state.db),
            Box::new(DataProviderMock::default()),
            params,
        );
        match opcode {
            opcodes::OpCode::CALL => it.run(),
            opcodes::OpCode::CALLCODE => it.run(),
            opcodes::OpCode::CALLDATACOPY => it.run(),
            opcodes::OpCode::CALLDATALOAD => it.run(),
            opcodes::OpCode::CALLDATASIZE => it.run(),
            opcodes::OpCode::CALLER => it.run(),
            opcodes::OpCode::CREATE => it.run(),
            opcodes::OpCode::CREATE2 => it.run(),
            _ => unimplemented!(),
        }
    }
}
