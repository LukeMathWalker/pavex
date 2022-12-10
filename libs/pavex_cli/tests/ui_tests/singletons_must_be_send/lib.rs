use pavex_builder::{f, AppBlueprint, Lifecycle};
use std::rc::Rc;

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

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::NonSendSingleton::new), Lifecycle::Singleton);
    bp
}
