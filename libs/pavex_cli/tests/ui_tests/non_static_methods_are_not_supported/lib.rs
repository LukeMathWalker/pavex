use pavex_builder::{f, Blueprint};

pub struct Streamer;

impl Streamer {
    pub fn stream_file(&self) -> pavex_runtime::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
