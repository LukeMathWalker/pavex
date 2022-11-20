use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct NonCloneSingleton;

impl NonCloneSingleton {
    pub fn new() -> NonCloneSingleton {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().constructor(f!(crate::NonCloneSingleton::new), Lifecycle::Singleton)
}
