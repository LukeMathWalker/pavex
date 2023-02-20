use std::path::PathBuf;

use pavex_builder::{f, Blueprint};

pub fn stream_file(_inner: PathBuf) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(f!(crate::stream_file), "/home");
    bp
}
