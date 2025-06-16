use pavex::blueprint::{from, Blueprint};

pub struct Streamer;

#[pavex::request_scoped]
pub fn alternative_logger() -> dep_55dca802::Logger {
    todo!()
}

#[pavex::methods]
impl Streamer {
    #[pavex::get(path = "/home")]
    pub fn stream_file(_logger: dep_55dca802::Logger) -> pavex::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(dep_55dca802::NEW_LOGGER);
    bp.constructor(ALTERNATIVE_LOGGER);
    bp.routes(from![crate]);
    bp
}
