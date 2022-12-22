use pavex_builder::{f, AppBlueprint};

pub struct Streamer;

impl Streamer {
    pub fn stream_file(&self) -> pavex_runtime::response::Response {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
