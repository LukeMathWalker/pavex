use pavex_builder::{f, AppBlueprint};

pub struct Streamer;

impl Streamer {
    pub fn stream_file() -> http::Response<hyper::body::Body> {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route(f!(crate::Streamer::stream_file), "/home")
}
