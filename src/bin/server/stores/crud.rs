use std::collections::HashMap;

use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

pub struct CrudStore<T: Clone> {
    entries: HashMap<Uuid, T>
}

impl<T: Default + Clone> Default for CrudStore<T> {
    fn default() -> Self {
        Self { entries: Default::default() }
    }
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

// Manual implementation of Serialize for CrudStore<T>
impl<T: Clone + Serialize> Serialize for CrudStore<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a struct with one field: "entries"
        let mut state = serializer.serialize_struct("CrudStore", 1)?;
        state.serialize_field("entries", &self.entries)?;
        state.end()
    }
}

// Manual implementation of Deserialize for CrudStore<T>
impl<'de, T: Clone + Deserialize<'de>> Deserialize<'de> for CrudStore<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Define a helper struct that mirrors the serialized form.
        #[derive(Deserialize)]
        struct CrudStoreData<T> {
            entries: HashMap<Uuid, T>,
        }
        let data = CrudStoreData::deserialize(deserializer)?;
        Ok(CrudStore { entries: data.entries })
    }
}
