use pavex_builder::{f, Blueprint, Lifecycle};

pub struct Logger;

pub fn new_logger() -> Logger {
    todo!()
}

pub struct Streamer;

impl Streamer {
    pub fn stream_file(_logger: Logger) -> pavex_runtime::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(crate::new_logger), Lifecycle::RequestScoped);
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
