use crate::runtime::AndroidApp;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static REGISTRY: RefCell<AppRegistry> = RefCell::new(AppRegistry::default());
}

#[derive(Default)]
struct AppRegistry {
    next_id: u64,
    apps: HashMap<u64, AndroidApp>,
}

impl AppRegistry {
    fn insert(&mut self, app: AndroidApp) -> u64 {
        let id = self.next_id.saturating_add(1).max(1);
        self.next_id = id;
        self.apps.insert(id, app);
        id
    }

    fn get_mut(&mut self, id: u64) -> Option<&mut AndroidApp> {
        self.apps.get_mut(&id)
    }

    fn remove(&mut self, id: u64) {
        self.apps.remove(&id);
    }
}

pub fn insert_app(app: AndroidApp) -> u64 {
    REGISTRY.with(|registry| registry.borrow_mut().insert(app))
}

pub fn remove_app(id: u64) {
    REGISTRY.with(|registry| registry.borrow_mut().remove(id));
}

pub fn with_app_mut<R>(id: u64, default: R, f: impl FnOnce(&mut AndroidApp) -> R) -> R {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let Some(app) = registry.get_mut(id) else {
            return default;
        };
        f(app)
    })
}
