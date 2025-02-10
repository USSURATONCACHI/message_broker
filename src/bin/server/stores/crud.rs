use std::collections::HashMap;

use uuid::Uuid;

#[derive(Default)]
pub struct CrudStore<T: Clone> {
    entries: HashMap<Uuid, T>
}

impl<T: Clone> CrudStore<T> {
    pub fn get(&self, uuid: Uuid) -> Option<T> {
        self.entries.get(&uuid).cloned()
    }

    pub fn get_all(&self) -> Vec<(Uuid, T)> {
        self.entries.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub fn remove(&mut self, uuid: Uuid) {
        self.entries.remove(&uuid);
    }

    pub fn create(&mut self, entry: T) -> Uuid {
        let uuid = Uuid::new_v4();
        self.entries.insert(uuid, entry);
        uuid
    }

    pub fn update(&mut self, uuid: Uuid, entry: T) {
        if self.entries.contains_key(&uuid) {
            self.entries.insert(uuid, entry);
        }
    }

    pub fn count(&self, predicate: impl Fn(&T) -> bool) -> usize {
        self.entries.iter().filter(|(_, elem)| predicate(*elem)).count()
    }
}