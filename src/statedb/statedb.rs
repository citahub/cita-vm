use ethereum_types::{Address, H256, U256};

/// StateDB is an EVM database for full state querying.
pub trait StateDB {
    fn create_account(&mut self, address: Address);

    fn sub_balance(&mut self, address: Address, value: U256);
    fn add_balance(&mut self, address: Address, value: U256);
    fn get_balance(&self, address: Address) -> U256;

    fn get_nonce(&self, address: Address) -> String;
    fn set_nonce(&mut self, address: Address, nonce: String);

    fn get_code_hash(&self, address: Address) -> H256;
    fn get_code(&self, address: Address) -> Vec<u8>;
    fn set_code(&mut self, address: Address, code: &[u8]);
    fn get_code_size(&self, address: Address) -> usize;

    fn add_refund(&mut self, quota: u64);
    fn set_refund(&mut self, quota: u64);
    fn get_refund(&self) -> u64;

    fn get_committed_state(&self, address: Address, hash: H256) -> H256;
    fn get_state(&self, address: Address, key: H256) -> H256;
    fn set_state(&mut self, address: Address, key: H256, value: H256);

    fn suicide(&mut self, address: Address) -> bool;
    fn has_suicided(&self, address: Address) -> bool;

    fn exists(&self, address: Address) -> bool;
    fn empty(&self, address: Address) -> bool;

    fn revert_to_snapshot(&mut self, id: usize);
    fn snapshot(&mut self) -> usize;

    fn add_log(&mut self, log: Log);
    fn add_preimage(&mut self, hash: H256, preimage: &[u8]);
}

pub struct Log {
    pub address: Address,
    pub topics: Vec<H256>,
    pub data: Vec<u8>,
    pub block_number: u64,
    pub tx_hash: H256,
    pub removed: bool,
}

pub struct ProofList([U256]);
