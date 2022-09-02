use pavex_builder::{f, AppBlueprint};
use std::path::PathBuf;

pub fn stream_file(_inner: PathBuf) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route(f!(crate::stream_file), "/home")
}
