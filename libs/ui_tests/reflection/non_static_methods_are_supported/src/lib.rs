use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub struct Streamer;

impl Streamer {
    pub fn new() -> Self {
        todo!()
    }

    pub fn stream_file(&self, logger: Logger) -> pavex::response::Response {
        todo!()
    }
}

#[derive(Clone)]
pub struct LoggerFactory;

pub struct Logger;

impl LoggerFactory {
    pub fn new() -> Self {
        todo!()
    }

    pub fn logger(&self) -> Logger {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Streamer::new), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::LoggerFactory::new), Lifecycle::Singleton);
    bp.constructor(f!(crate::LoggerFactory::logger), Lifecycle::Transient);
    bp.route(GET, "/home", f!(crate::Streamer::stream_file));
    bp
}
