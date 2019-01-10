// CITA
// Copyright 2016-2018 Cryptape Technologies LLC.

// This program is free software: you can redistribute it
// and/or modify it under the terms of the GNU General Public
// License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any
// later version.

// This program is distributed in the hope that it will be
// useful, but WITHOUT ANY WARRANTY; without even the implied
// warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use cita_types::{H256, U256};
use std::collections::HashMap;
use std::sync::Arc;
use util::Bytes;
use cita_trie::trie::{PatriciaTrie, Trie};
use cita_trie::db::MemoryDB;
use cita_trie::codec::RLPNodeCodec;
use cita_trie::trie::TrieResult;
use lru_cache::LruCache;
use std::cell::RefCell;
use hashable::{Hashable, HASH_EMPTY, HASH_NULL_RLP};

const STORAGE_CACHE_ITEMS: usize = 8192;

pub struct Account {
    balance: U256,
    nonce: U256,

    storage_root: H256,
    storage_cache: RefCell<LruCache<H256, H256>>,
    storage: HashMap<H256, H256>,

    code: Arc<Bytes>, 
    code_size: Option<usize>,     
    code_hash: H256,
    
    abi: Arc<Bytes>, 
    abi_size: Option<usize>,    
    abi_hash: H256,
    
    address_hash: Option<H256>,
}

impl Account {

    pub fn new(
        balance: U256,
        nonce: U256,
        storage: HashMap<H256, H256>,
        code: Bytes,
        abi: Bytes,
    ) -> Account {
        Account {
            balance,
            nonce,
            storage_root: HASH_NULL_RLP,
            storage_cache: Self::empty_storage_cache(),
            storage: storage,
            code: Arc::new(code),
            code_size: Some(code.len()),
            code_hash: code.cryp_hash(),
            abi: Arc::new(abi),
            abi_size: Some(abi.len()),
            abi_hash: abi.crypt_hash(),
            address_hash: None,
        }
    }

    fn empty_storage_cache() -> RefCell<LruCache<H256, H256>> {
        RefCell::new(LruCache::new(STORAGE_CACHE_ITEMS))
    }
    
    /// Get balance of the account
    pub fn balance(&self) -> &U256 {
        &self.balance
    }

    /// Get nonce of the account
    pub fn nonce(&self) -> &U256 {
       &self.nonce
    }
    
    /// Get code of the account
    pub fn code(&self) -> Option<Arc<Bytes>> {
       if self.code_hash != HASH_EMPTY && self.code.is_empty() {
           return None;
       }
       Some(Arc::clone(&self.code))
    }

    /// Get code hash of the account
    pub fn code_hash(&self) -> &H256{
        &self.code_hash
    }

    /// Get code size of the account
    pub fn code_size(&self) -> Option<usize>{
        self.code_size
    }

    /// Get abi of the account
    pub fn abi(&self) -> Option<Arc<Bytes>> {
       if self.abi_hash != HASH_EMPTY && self.abi.is_empty() {
           return None;
       }
       Some(Arc::clone(&self.abi))
    }

    /// Get abi hash of the account
    pub fn abi_hash(&self) -> &H256{
       &self.abi_hash
    }

    /// Get abi size of the account
    pub fn abi_size(&self) -> Option<usize> {
        self.abi_size
    }

    /// Whether storage of the account is null
    pub fn storage_is_null(&self) -> bool {
        self.storage.is_empty()
    }

    /// Get the storage of the account
    pub fn storage(&self) -> &HashMap<H256, H256> {
        &self.storage
    }

    /// Get storage root of the account
    pub fn storage_root(&self) -> Option<&H256>{
        if self.storage_is_null() {
            Some(&self.storage_root)
        } else {
            None
        }
    }

    /// Increase nonce of the account by one
    pub fn increase_nonce(&mut self) {
        self.nonce = self.nonce + U256::from(1u8);
    }

    /// Increase account balance
    pub fn add_balance(&mut self, x: &U256) {
        self.balance = self.balance.saturating_add(*x);
    }

    /// Decrease account balance
    pub fn sub_balance(&mut self, x:&U256) {
        self.balance = self.balance.saturating_sub(*x);
    }

    // Commit the storage and update storage_root
    pub fn commit_storage(&self) {
        // db先写里边吧
        let mut memdb = MemoryDB::new();

        let mut trie = PatriciaTrie::from(&mut memdb, RLPNodeCodec::default(), self.storage_root.0).unwrap();
        for (k, v) in self.storage.drain() {
            if v.is_zero() {
                trie.remove(&k);
            } else {
                trie.insert(&k,&v);
            }
            self.storage_cache.borrow_mut().insert(k, v);
        }
    }
}