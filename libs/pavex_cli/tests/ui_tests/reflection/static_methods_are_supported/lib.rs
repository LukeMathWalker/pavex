use pavex_builder::{f, router::GET, Blueprint};

pub struct Streamer;

impl Streamer {
    pub fn stream_file() -> pavex::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::Streamer::stream_file));
    bp
}
