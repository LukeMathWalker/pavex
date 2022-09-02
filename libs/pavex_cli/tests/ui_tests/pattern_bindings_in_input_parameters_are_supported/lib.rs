use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Streamer {
    pub a: usize,
    pub b: isize,
}

pub fn streamer() -> Streamer {
    todo!()
}

pub fn stream_file(Streamer { a, b }: Streamer) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::streamer), Lifecycle::Singleton)
        .route(f!(crate::stream_file), "/home")
}
