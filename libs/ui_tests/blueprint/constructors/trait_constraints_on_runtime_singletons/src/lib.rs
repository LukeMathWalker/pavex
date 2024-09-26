use std::rc::Rc;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub struct NonSendSingleton(Rc<()>);

impl Clone for NonSendSingleton {
    fn clone(&self) -> NonSendSingleton {
        Self(Rc::clone(&self.0))
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

impl NonSyncSingleton {
    pub fn new() -> NonSyncSingleton {
        todo!()
    }
}

pub fn handler(_s: &NonSendSingleton, _a: &NonSyncSingleton) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::NonSendSingleton::new));
    bp.singleton(f!(crate::NonSyncSingleton::new));
    // The handler is needed because bounds are only checked for singletons
    // that are used at runtime
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
