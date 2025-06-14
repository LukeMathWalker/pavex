use pavex::blueprint::{constructor::Lifecycle, from, Blueprint};
use pavex::f;

pub struct Logger;

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
    bp.constructor(f!(crate::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(crate::new_logger), Lifecycle::RequestScoped);
    bp.routes(from![crate]);
    bp
}
