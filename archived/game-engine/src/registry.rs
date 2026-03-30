use crate::engine::GameEngine;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
pub struct EngineRegistry {
    next_id: u64,
    engines: HashMap<u64, GameEngine>,
}

static REGISTRY: OnceLock<Mutex<EngineRegistry>> = OnceLock::new();

impl EngineRegistry {
    pub fn global() -> &'static Mutex<EngineRegistry> {
        REGISTRY.get_or_init(|| {
            Mutex::new(EngineRegistry {
                next_id: 1,
                engines: HashMap::new(),
            })
        })
    }

    pub fn insert(&mut self, engine: GameEngine) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.engines.insert(id, engine);
        id
    }

    pub fn remove(&mut self, id: u64) {
        self.engines.remove(&id);
    }

    pub fn get(&self, id: u64) -> Option<&GameEngine> {
        self.engines.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut GameEngine> {
        self.engines.get_mut(&id)
    }
}
