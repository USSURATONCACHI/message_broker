use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::Handle;

#[derive(Debug, Default)]
pub struct StoreRegistry {
    handles: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl StoreRegistry {
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
        }
    }

    pub fn add<T: 'static + Send + Sync>(&mut self, handle: Handle<T>) {
        let type_id = TypeId::of::<T>();
        if self.handles.contains_key(&type_id) {
            panic!("Store for type {} already registered", std::any::type_name::<T>());
        }
        self.handles.insert(type_id, Box::new(handle));
    }

    pub fn get<T: 'static>(&self) -> Handle<T> {
        let type_id = TypeId::of::<T>();
        self.handles.get(&type_id)
            .and_then(|boxed| boxed.downcast_ref::<Handle<T>>())
            .cloned()
            .unwrap_or_else(|| panic!("Store {} not registered", std::any::type_name::<T>()))
    }
}