use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct NonCloneSingleton;

impl NonCloneSingleton {
    pub fn new() -> NonCloneSingleton {
        todo!()
    }
}

pub fn handler(_s: NonCloneSingleton) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::NonCloneSingleton::new), Lifecycle::Singleton);
    // The handler is needed because bounds are only checked for singletons
    // that are used at runtime
    bp.route(f!(crate::handler), "/home");
    bp
}
