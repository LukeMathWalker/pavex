use pavex_builder::{f, AppBlueprint, Lifecycle};
use std::rc::Rc;

pub struct NonSyncSingleton(std::sync::mpsc::Sender<()>);

impl Clone for NonSyncSingleton {
    fn clone(&self) -> NonSyncSingleton {
        Self(self.0.clone())
    }
}

impl NonSyncSingleton {
    pub fn new() -> NonSyncSingleton {
        todo!()
    }
}

pub fn owned_handler(input: NonSyncSingleton) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn ref_handler(input: &NonSyncSingleton) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::NonSyncSingleton::new), Lifecycle::Singleton)
        .route(f!(crate::ref_handler), "/ref")
        .route(f!(crate::owned_handler), "/owned")
}
