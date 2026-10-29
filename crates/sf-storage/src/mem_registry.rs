//! In-memory lock table used by the Slice 1 proxy. LanceDB replaces the loader in Slice 3.

use sf_kernels::DomainHypercubeLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct MemoryRegistry {
    locks_by_id: HashMap<String, Arc<DomainHypercubeLock>>,
}

impl MemoryRegistry {
    pub fn from_locks(locks: impl IntoIterator<Item = DomainHypercubeLock>) -> Self {
        let mut locks_by_id = HashMap::new();
        for lock in locks {
            locks_by_id.insert(lock.domain_id.clone(), Arc::new(lock));
        }
        Self { locks_by_id }
    }

    pub fn get(&self, domain_id: &str) -> Option<Arc<DomainHypercubeLock>> {
        self.locks_by_id.get(domain_id).cloned()
    }

    pub fn len(&self) -> usize {
        self.locks_by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.locks_by_id.is_empty()
    }

    pub fn locks(&self) -> Vec<Arc<DomainHypercubeLock>> {
        self.locks_by_id.values().cloned().collect()
    }
}
