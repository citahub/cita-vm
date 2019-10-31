use std::sync::Arc;

use ethereum_types::{Address, U256};

use crate::executive::{BlockDataProvider, BlockDataProviderMock, Config, Executive};
use crate::state;

// Use FakeVM for testing!
pub struct FakeVM {
    pub account1: Address,
    pub account2: Address,
    pub executor: Executive<state::MemoryDB>,
}

impl FakeVM {
    pub fn new() -> Self {
        let account1 = Address::from("0x0000000000000000000000000000000000000001");
        let account2 = Address::from("0x0000000000000000000000000000000000000002");

        let db = Arc::new(crate::state::MemoryDB::new(false));
        let mut state = state::State::new(db.clone()).unwrap();
        state.new_contract(&account1, U256::from(100_000_000_000u64), U256::from(1), vec![]);
        state.new_contract(&account2, U256::from(200_000_000_000u64), U256::from(1), vec![]);
        state.commit().unwrap();

        let block_data_provider: Arc<dyn BlockDataProvider> = Arc::new(BlockDataProviderMock::default());

        Self {
            account1,
            account2,
            executor: Executive::new(block_data_provider, state, Config::default()),
        }
    }
}

impl Default for FakeVM {
    fn default() -> Self {
        FakeVM::new()
    }
}
