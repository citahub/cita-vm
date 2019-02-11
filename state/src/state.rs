use super::account::Account;
use super::account_entry::{AccountEntry, AccountState};
use cita_trie::codec::RLPNodeCodec;
use cita_trie::db::MemoryDB;
use cita_trie::trie::PatriciaTrie;
use cita_trie::trie::Trie;
use ethereum_types::{Address, H256, U256};
use evm::interpreter::Context;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

pub struct State {
    pub db: MemoryDB,
    pub root: H256,
    pub cache: RefCell<HashMap<Address, AccountEntry>>,
    pub checkpoints: RefCell<Vec<HashMap<Address, Option<AccountEntry>>>>,
    pub context: Context,
    pub db_origin: BTreeMap<Address, Account>,
    pub refund: BTreeMap<Address, u64>,
}

impl State {
    pub fn new(mut db: MemoryDB) -> State {
        let mut trie = PatriciaTrie::new(&mut db, RLPNodeCodec::default());
        let root = trie.root().unwrap();
        State {
            db,
            root: H256::from_slice(&root),
            cache: RefCell::new(HashMap::new()),
            checkpoints: RefCell::new(Vec::new()),
            context: Context::default(),
            db_origin: BTreeMap::new(),
            refund: BTreeMap::new(),
        }
    }

    pub fn new_contract(&mut self, contract: &Address, balance: U256, nonce: U256) {
        self.insert_cache(
            contract,
            AccountEntry::new_dirty_account(Some(Account::new_contract(balance, nonce))),
        );
    }

    pub fn kill_contract(&mut self, contract: &Address) {
        self.insert_cache(contract, AccountEntry::new_dirty_account(None));
    }

    pub fn is_empty(&mut self, address: &Address) -> bool {
        if let Some(account_entry) = self.cache.borrow().get(address) {
            if let Some(ref _account) = account_entry.account {
                return true;
            }
        }

        let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0).unwrap();
        if let Some(ref _account) = trie.get(&address).unwrap() {
            return true;
        }
        false
    }

    pub fn db(self) -> MemoryDB {
        self.db
    }

    pub fn root(&self) -> &H256 {
        &self.root
    }

    pub fn context(self) -> Context {
        self.context
    }

    pub fn add_refund(&mut self, address: &Address, n: u64) {
        match self.ensure_cached(address) {
            Some(mut account) => {
                account.add_balance(&U256::from(n));
                self.insert_cache(address, AccountEntry::new_dirty_account(Some(account)))
            }
            None => {
                self.new_contract(address, U256::from(n), U256::from(0));
            }
        }

        self.refund
            .entry(*address)
            .and_modify(|v| *v += n)
            .or_insert(n);
    }

    pub fn sub_refund(&mut self, address: &Address, n: u64) {
        match self.ensure_cached(address) {
            Some(mut account) => {
                account.sub_balance(&U256::from(n));
                self.insert_cache(address, AccountEntry::new_dirty_account(Some(account)))
            }
            None => {
                self.new_contract(address, U256::from(n), U256::from(0));
            }
        }

        self.refund
            .entry(*address)
            .and_modify(|v| *v -= n)
            .or_insert(n);
    }

    pub fn ensure_cached(&mut self, address: &Address) -> Option<Account> {
        if let Some(account_entry) = self.cache.borrow().get(address) {
            if let Some(account) = &account_entry.account {
                return Some((*account).clone_all());
            }
        }

        let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0).unwrap();
        match trie.get(&address) {
            Ok(Some(account_rlp)) => {
                let account = Account::from_rlp(&account_rlp);
                self.insert_cache(
                    address,
                    AccountEntry::new_clean_account(Some(account.clone_basic())),
                );
                return Some(account);
            }
            Ok(None) => {
                // TODO
            }
            Err(_) => {
                // TODO
            }
        }
        None
    }

    pub fn storage_at(&mut self, address: &Address, key: &H256) -> H256 {
        if let Some(mut account) = self.ensure_cached(address) {
            if let Some(value) = account.cached_storage_at(key) {
                return value;
            }
            if let Some(value) = account.trie_storage_at(&mut self.db, key) {
                return value;
            }
        }
        H256::from(0)
    }

    pub fn set_storage(&mut self, address: &Address, key: H256, value: H256) {
        if self.storage_at(address, &key) != value {
            let contain_key = self.cache.borrow().contains_key(address);
            if !contain_key {
                let trie = PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0)
                    .unwrap();
                match trie.get(&address) {
                    Ok(rlp) => {
                        let mut account = Account::from_rlp(&rlp.unwrap());
                        account.set_storage(key, value);
                        self.insert_cache(address, AccountEntry::new_clean_account(Some(account)));
                    }
                    Err(_) => panic!("this account is not exist in patriciaTrie."),
                }
            }
        }
        self.add_checkpoint(address);

        if let Some(ref mut account_entry) = self.cache.borrow_mut().get_mut(address) {
            match account_entry.account {
                Some(ref mut account) => {
                    account.set_storage(key, value);
                    account_entry.state = AccountState::Dirty;
                }
                None => panic!("account always exist in cache."),
            }
        }
    }

    pub fn insert_cache(&self, address: &Address, account_entry: AccountEntry) {
        let is_dirty = account_entry.is_dirty();
        self.cache
            .borrow_mut()
            .insert(*address, account_entry.clone_dirty_account_entry());

        if is_dirty {
            if let Some(checkpoint) = self.checkpoints.borrow_mut().last_mut() {
                checkpoint.entry(*address).or_insert(Some(account_entry));
            }
        }
    }

    pub fn commit(&mut self) {
        assert!(self.checkpoints.borrow().is_empty());
        let mut trie =
            PatriciaTrie::from(&mut self.db, RLPNodeCodec::default(), &self.root.0).unwrap();

        for (address, a) in self
            .cache
            .borrow_mut()
            .iter_mut()
            .filter(|&(_, ref a)| a.is_dirty())
        {
            a.state = AccountState::Committed;
            match a.account {
                Some(ref mut account) => {
                    trie.insert(address, &account.rlp());
                }
                None => {
                    trie.remove(address);
                }
            }
        }
    }

    pub fn checkpoint(&mut self) {
        self.checkpoints.borrow_mut().push(HashMap::new());
    }

    fn add_checkpoint(&self, address: &Address) {
        if let Some(ref mut checkpoint) = self.checkpoints.borrow_mut().last_mut() {
            checkpoint.entry(*address).or_insert_with(|| {
                self.cache
                    .borrow()
                    .get(address)
                    .map(AccountEntry::clone_dirty_account_entry)
            });
        }
    }

    // If the transaction if executed successfully
    pub fn discard_checkpoint(&mut self) {
        let last = self.checkpoints.borrow_mut().pop();
        if let Some(mut checkpoint) = last {
            if let Some(prev) = self.checkpoints.borrow_mut().last_mut() {
                if prev.is_empty() {
                    *prev = checkpoint;
                } else {
                    for (k, v) in checkpoint.drain() {
                        prev.entry(k).or_insert(v);
                    }
                }
            }
        }
    }

    // If the transaction fails to execute
    pub fn revert_checkpoint(&mut self) {
        if let Some(mut last) = self.checkpoints.borrow_mut().pop() {
            for (k, v) in last.drain() {
                match v {
                    Some(v) => match self.cache.get_mut().entry(k) {
                        Entry::Occupied(mut e) => {
                            e.get_mut().overwrite_with_account_entry(v);
                        }
                        Entry::Vacant(e) => {
                            e.insert(v);
                        }
                    },
                    None => {
                        if let Entry::Occupied(e) = self.cache.get_mut().entry(k) {
                            if e.get().is_dirty() {
                                e.remove();
                            }
                        }
                    }
                }
            }
        }
    }
}
