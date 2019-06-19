use super::account::StateObject;

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum ObjectStatus {
    Clean,
    Dirty,
    Committed,
}

#[derive(Debug, Clone)]
pub struct StateObjectEntry {
    /// State object entry. `None` if account known to be non-existant.
    pub state_object: Option<StateObject>,
    pub status: ObjectStatus,
}

impl StateObjectEntry {
    /// Create a new entry with status `Clean`.
    pub fn new_clean(state_object: Option<StateObject>) -> StateObjectEntry {
        StateObjectEntry {
            state_object,
            status: ObjectStatus::Clean,
        }
    }

    /// Create a new entry with status `Dirty`.
    pub fn new_dirty(state_object: Option<StateObject>) -> StateObjectEntry {
        StateObjectEntry {
            state_object,
            status: ObjectStatus::Dirty,
        }
    }

    /// Return true is status == Dirty.
    pub fn is_dirty(&self) -> bool {
        self.status == ObjectStatus::Dirty
    }

    /// Clone dirty data into new `ObjectEntry`. This includes
    /// account data and modified storage keys.
    pub fn clone_dirty(&self) -> StateObjectEntry {
        StateObjectEntry {
            state_object: self.state_object.as_ref().map(StateObject::clone_dirty),
            status: self.status.clone(),
        }
    }

    /// Merge with others.
    pub fn merge(&mut self, other: StateObjectEntry) {
        self.status = other.status;
        match other.state_object {
            Some(acc) => {
                if let Some(ref mut ours) = self.state_object {
                    ours.merge(acc);
                } else {
                    self.state_object = Some(acc);
                }
            }
            None => self.state_object = None,
        }
    }
}
