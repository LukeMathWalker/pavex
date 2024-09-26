use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub struct Logger;

pub fn new_logger() -> Logger {
    todo!()
}

pub struct Streamer;

impl Streamer {
    pub fn stream_file(_logger: Logger) -> pavex::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(crate::new_logger), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::Streamer::stream_file));
    bp
}
