use pavex::blueprint::{constructor::Lifecycle, from, Blueprint};

pub struct Logger;

#[pavex::singleton]
pub fn new_logger() -> Logger {
    todo!()
}

pub struct Streamer;

#[pavex::methods]
impl Streamer {
    #[pavex::get(path = "/home")]
    pub fn stream_file(_logger: Logger) -> pavex::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(NEW_LOGGER);
    // Register *again*, but with a different lifecycle
    bp.constructor(NEW_LOGGER)
        .lifecycle(Lifecycle::RequestScoped);
    bp.routes(from![crate]);
    bp
}
