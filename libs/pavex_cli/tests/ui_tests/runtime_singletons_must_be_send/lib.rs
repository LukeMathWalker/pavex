use std::rc::Rc;

use pavex_builder::{f, AppBlueprint, Lifecycle};

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

pub fn handler(_s: NonSendSingleton) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::NonSendSingleton::new), Lifecycle::Singleton);
    // The handler is needed because bounds are only checked for singletons
    // that are used at runtime
    bp.route(f!(crate::handler), "/home");
    bp
}
