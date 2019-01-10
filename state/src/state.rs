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

use cita_types::{Address, H256};
// use cita_trie::trie::PatriciaTrie;
// use std::cell::RefCell;
// use std::collections::HashMap;

pub struct State {
    root: H256,
    // db: Box<JournalDB>,
    // checkpoints: RefCell<Vec<HashMap<Address, Option<AccountEntry>>>>,   // type
}

impl State {
        
    /// Create new state with empty state root
    pub fn new() {

    }
 
    /// Create a recoverable checkpoint of state
    pub fn checkpoint() {

    }

    /// Discard checkpoint (more docs!)
    pub fn discard_checkpoint() {

    }

    /// Revert to checkpoint (more docs!)
    pub fn revert_to_checkpoint() {

    }

    /// Return root reference
    pub fn root() {

    }
    
    /// Remove an existing account
    pub fn kill_account() {

    }

    /// Whether an account exists
    pub fn exists() {

    }

    /// Get the balance of an account
    pub fn balance() {

    }
    
    /// Get the nonce of an account
    pub fn nonce() {

    }

    /// Get the storage root of an account
    pub fn storage_root() {

    }

    /// Set the contents of the trie's storage
    pub fn set_storage() {

    }

    /// Get the contents of the trie's storage at `key` 
    pub fn storage_at() {

    }

    /// Get account code
    pub fn code() {

    }

    /// Get account code hash
    pub fn code_hash() {

    }

    /// Get account code size
    pub fn code_size() {

    }
    
    /// Get account abi
    pub fn abi() {

    }

    /// Get account abi hash
    pub fn abi_hash() {

    }

    /// Get account abi size
    pub fn abi_size() {

    }
}