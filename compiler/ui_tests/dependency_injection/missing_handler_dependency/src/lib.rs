use std::path::PathBuf;

use pavex::{blueprint::from, Blueprint};

#[pavex::get(path = "/home")]
pub fn stream_file(_inner: PathBuf) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
