use std::path::PathBuf;

use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn stream_file(_inner: PathBuf) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
