use std::path::PathBuf;

use pavex_builder::{f, AppBlueprint};

pub fn stream_file(_inner: PathBuf) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::stream_file), "/home");
    bp
}
