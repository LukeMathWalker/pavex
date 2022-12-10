use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Logger;

pub fn new_logger() -> Logger {
    todo!()
}

pub struct Streamer;

impl Streamer {
    pub fn stream_file(_logger: Logger) -> http::Response<hyper::body::Body> {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(crate::new_logger), Lifecycle::RequestScoped);
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
