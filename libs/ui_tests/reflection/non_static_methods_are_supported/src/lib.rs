use pavex::{blueprint::from, Blueprint};

pub struct Streamer;

impl Default for Streamer {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl Streamer {
    #[request_scoped]
    pub fn new() -> Self {
        todo!()
    }

    #[get(path = "/home")]
    pub fn stream_file(&self, _logger: Logger) -> pavex::Response {
        todo!()
    }
}

#[derive(Clone)]
pub struct LoggerFactory;

pub struct Logger;

impl Default for LoggerFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl LoggerFactory {
    #[singleton]
    pub fn new() -> Self {
        todo!()
    }

    #[transient]
    pub fn logger(&self) -> Logger {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
