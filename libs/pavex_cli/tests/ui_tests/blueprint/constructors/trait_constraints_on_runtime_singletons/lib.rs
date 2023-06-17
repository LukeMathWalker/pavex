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

pub struct NonCloneSingleton;

impl NonCloneSingleton {
    pub fn new() -> NonCloneSingleton {
        todo!()
    }
}

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

pub fn handler(
    _s: NonSendSingleton,
    _a: NonSyncSingleton,
    _c: NonCloneSingleton,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::NonSendSingleton::new), Lifecycle::Singleton);
    bp.constructor(f!(crate::NonCloneSingleton::new), Lifecycle::Singleton);
    bp.constructor(f!(crate::NonSyncSingleton::new), Lifecycle::Singleton);
    // The handler is needed because bounds are only checked for singletons
    // that are used at runtime
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
