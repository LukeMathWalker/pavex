use std::rc::Rc;

use pavex::blueprint::{from, Blueprint};

pub struct NonSendSingleton(Rc<()>);

impl Clone for NonSendSingleton {
    fn clone(&self) -> NonSendSingleton {
        Self(Rc::clone(&self.0))
    }
}

#[pavex::methods]
impl Default for NonSendSingleton {
    #[singleton]
    fn default() -> Self {
        Self::new()
    }
}

impl NonSendSingleton {
    pub fn new() -> NonSendSingleton {
        todo!()
    }
}

pub struct NonSyncSingleton(std::cell::Cell<()>);

impl Clone for NonSyncSingleton {
    fn clone(&self) -> NonSyncSingleton {
        Self(self.0.clone())
    }
}

impl Default for NonSyncSingleton {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl NonSyncSingleton {
    #[singleton]
    pub fn new() -> NonSyncSingleton {
        todo!()
    }
}

#[pavex::get(path = "/home")]
pub fn handler(_s: &NonSendSingleton, _a: &NonSyncSingleton) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    // The handler is needed because bounds are only checked for singletons
    // that are used at runtime
    bp.routes(from![crate]);
    bp
}
