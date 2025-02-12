use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct StoreRegistry {
    stores: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl StoreRegistry {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    pub fn add<T: 'static + Send + Sync>(&mut self, store: T) {
        let type_id = TypeId::of::<T>();
        if self.stores.contains_key(&type_id) {
            panic!("Store for type {} already registered", std::any::type_name::<T>());
        }
        self.stores.insert(type_id, Box::new(store));
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> &T {
        let type_id = TypeId::of::<T>();
        self.stores.get(&type_id)
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .unwrap_or_else(|| panic!("Store {} not registered", std::any::type_name::<T>()))
    }
}