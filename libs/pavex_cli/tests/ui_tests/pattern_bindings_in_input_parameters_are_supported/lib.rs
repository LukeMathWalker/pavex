use pavex_builder::{f, AppBlueprint, Lifecycle};

#[derive(Clone)]
pub struct Streamer {
    pub a: usize,
    pub b: isize,
}

pub fn streamer() -> Streamer {
    todo!()
}

pub fn stream_file(Streamer { a, b }: Streamer) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::streamer), Lifecycle::Singleton);
    bp.route(f!(crate::stream_file), "/home");
    bp
}
