use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Streamer;

impl Streamer {
    pub fn stream_file(_logger: dep::Logger) -> http::Response<hyper::body::Body> {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(dep::new_logger), Lifecycle::Singleton)
        .constructor(f!(::dep::new_logger), Lifecycle::RequestScoped)
        .route(f!(crate::Streamer::stream_file), "/home")
}
