use pavex_builder::{f, AppBlueprint};

pub struct Streamer;

impl Streamer {
    pub fn stream_file(&self) -> http::Response<hyper::body::Body> {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
