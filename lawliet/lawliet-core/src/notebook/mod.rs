use indexmap::IndexMap;

use crate::common::{ActorKey, AttemptCount, ChannelKey};

#[derive(Debug)]
pub enum NotebookError {
    NoOwner,
    OnCooldown,
    NotOwned,
}

#[derive(Debug, PartialEq)]
pub struct Notebook {
    pub volatile: bool, // if a notebook is volatile, it will be destroyed when the original owner's
    // role changes
    pub fake: bool, // if a notebook is fake, it cannot actually kill people
    pub original_owner: Option<ActorKey>,
    pub dormant_true_owner: Option<ActorKey>, // the person who the notebook should return to if it was
    // discovered that they never actually died (this should be None most of the time, but
    // pseudocide will set it to the target's ID if applicable)
    pub owner: Option<ActorKey>, // the person this notebook currently belongs to
    pub borrowed: Option<ActorKey>, // the person the notebook is being borrowed from (if any)
    pub iteration_successes: IndexMap<ActorKey, AttemptCount>, // success counts (correct names)
    pub iteration_failures: IndexMap<ActorKey, AttemptCount>, // failed counts (wrong names)
    pub channel_id: ChannelKey,
}

impl Notebook {
    pub fn new(channel_id: ChannelKey, fake: bool) -> Self {
        Notebook {
            fake,
            volatile: false,
            dormant_true_owner: None,
            original_owner: None,
            owner: None,
            borrowed: None,
            iteration_successes: IndexMap::new(),
            iteration_failures: IndexMap::new(),
            channel_id,
        }
    }

    pub fn set_original_owner(&mut self, id: ActorKey) {
        self.original_owner = Some(id);
    }

    /// gives the notebook to the person AND sets them as a the true owner
    pub fn set_true_owner(&mut self, id: ActorKey, volatile: bool) {
        self.volatile = volatile;
        if self.original_owner.is_none() {
            self.set_original_owner(id);
        }
        self.borrowed = None;
        self.owner = Some(id);
    }

    /// set the dormant true owner to the current true owner
    pub fn set_dormant(&mut self) {
        self.dormant_true_owner = self.get_true_owner()
    }

    pub fn awaken_dormant_owner(&mut self) {
        if let Some(dormant_owner) = self.dormant_true_owner {
            self.set_true_owner(dormant_owner, self.volatile);
        }
        self.dormant_true_owner = None;
    }

    pub fn get_dormant_owner(&self) -> Option<ActorKey> {
        self.dormant_true_owner
    }

    pub fn strip_ownership(&mut self) {
        self.volatile = false;
        self.borrowed = None;
        self.owner = None;
    }

    pub fn get_true_owner(&self) -> Option<ActorKey> {
        if let Some(borrowed) = self.borrowed {
            Some(borrowed)
        } else {
            self.owner
        }
    }

    pub fn can_lend(&self, id: ActorKey) -> Result<(), NotebookError> {
        if self.owner != Some(id) {
            return Err(NotebookError::NotOwned);
        }
        if self.borrowed.is_some() {
            return Err(NotebookError::NotOwned);
        }
        Ok(())
    }

    pub fn lend(&mut self, id: ActorKey) -> Result<(), NotebookError> {
        if let Some(owner) = self.owner {
            self.borrowed = Some(owner);
            self.owner = Some(id);
            Ok(())
        } else {
            Err(NotebookError::NoOwner)
        }
    }

    pub fn is_owner_borrowing(&self) -> bool {
        self.borrowed.is_some()
    }

    pub fn iteration_reset(&mut self) {
        self.iteration_successes = IndexMap::new();
        self.iteration_failures = IndexMap::new();
        if let Some(true_owner) = self.borrowed {
            self.set_true_owner(true_owner, self.volatile);
        }
    }

    pub fn on_write_success(&mut self, id: ActorKey) {
        self.iteration_successes
            .entry(id)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    pub fn on_write_failure(&mut self, id: ActorKey) {
        self.iteration_failures
            .entry(id)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    pub fn failures_remaining(&self, id: ActorKey, limit: AttemptCount) -> AttemptCount {
        limit - self.iteration_failures.get(&id).unwrap_or(&0)
    }

    pub fn successes_remaining(&self, id: ActorKey, limit: AttemptCount) -> AttemptCount {
        limit - self.iteration_successes.get(&id).unwrap_or(&0)
    }

    pub fn can_write(
        &self,
        id: ActorKey,
        fail_limit: u16,
        success_limit: u16,
    ) -> Result<(), NotebookError> {
        if self.owner != Some(id) {
            return Err(NotebookError::NotOwned);
        }

        if self.iteration_failures.get(&id).copied().unwrap_or(0) >= fail_limit {
            return Err(NotebookError::OnCooldown);
        }
        if self.iteration_successes.get(&id).copied().unwrap_or(0) >= success_limit {
            return Err(NotebookError::OnCooldown);
        }

        Ok(())
    }
}
