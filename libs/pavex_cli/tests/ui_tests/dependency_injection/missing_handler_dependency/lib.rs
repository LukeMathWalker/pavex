use std::path::PathBuf;

use pavex_builder::{f, router::GET, Blueprint};

pub fn stream_file(_inner: PathBuf) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
