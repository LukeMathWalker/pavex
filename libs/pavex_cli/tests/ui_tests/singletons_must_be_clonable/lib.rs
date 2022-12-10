use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct NonCloneSingleton;

impl NonCloneSingleton {
    pub fn new() -> NonCloneSingleton {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::NonCloneSingleton::new), Lifecycle::Singleton);
    bp
}
