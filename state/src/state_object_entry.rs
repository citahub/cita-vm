use super::account::StateObject;

#[derive(Eq, PartialEq, Clone)]
pub enum ObjectStatus {
    Clean,
    Dirty,
    Committed,
}

pub struct StateObjectEntry {
    pub state_object: Option<StateObject>,
    pub status: ObjectStatus,
}

impl StateObjectEntry {
    pub fn is_dirty(&self) -> bool {
        self.status == ObjectStatus::Dirty
    }

    pub fn state_object(&self) -> Option<&StateObject> {
        self.state_object.as_ref()
    }

    pub fn new_clean_state_object(state_object: Option<StateObject>) -> StateObjectEntry {
        StateObjectEntry {
            state_object,
            status: ObjectStatus::Clean,
        }
    }

    pub fn new_dirty_state_object(state_object: Option<StateObject>) -> StateObjectEntry {
        StateObjectEntry {
            state_object,
            status: ObjectStatus::Dirty,
        }
    }

    pub fn clone_dirty_state_object_entry(&self) -> StateObjectEntry {
        StateObjectEntry {
            state_object: self
                .state_object
                .as_ref()
                .map(StateObject::clone_dirty_state_object),
            status: self.status.clone(),
        }
    }

    pub fn overwrite_with_state_object_entry(&mut self, other: StateObjectEntry) {
        self.status = other.status;
        match other.state_object {
            Some(acc) => {
                if let Some(ref mut ours) = self.state_object {
                    ours.overwrite_with_state_object(acc);
                }
            }
            None => self.state_object = None,
        }
    }
}
