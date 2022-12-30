use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Logger;

pub fn stream_file(input: (usize, isize)) -> Logger {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::stream_file), Lifecycle::Singleton);
    bp
}
