use super::account::Account;

#[derive(Eq, PartialEq, Clone)]
pub enum AccountState {
    Clean,
    Dirty,
    Committed,
}

pub struct AccountEntry {
    pub account: Option<Account>,
    pub state: AccountState,
}

impl AccountEntry {
    pub fn is_dirty(&self) -> bool {
        self.state == AccountState::Dirty
    }

    pub fn account(&self) -> Option<&Account> {
        self.account.as_ref()
    }

    pub fn new_clean_account(account: Option<Account>) -> AccountEntry {
        AccountEntry {
            account,
            state: AccountState::Clean,
        }
    }

    pub fn new_dirty_account(account: Option<Account>) -> AccountEntry {
        AccountEntry {
            account,
            state: AccountState::Dirty,
        }
    }

    pub fn clone_dirty_account_entry(&self) -> AccountEntry {
        AccountEntry {
            account: self.account.as_ref().map(Account::clone_dirty_account),
            state: self.state.clone(),
        }
    }

    pub fn overwrite_with_account_entry(&mut self, other: AccountEntry) {
        self.state = other.state;
        match other.account {
            Some(acc) => {
                if let Some(ref mut ours) = self.account {
                    ours.overwrite_with_account(acc);
                }
            }
            None => self.account = None,
        }
    }
}
