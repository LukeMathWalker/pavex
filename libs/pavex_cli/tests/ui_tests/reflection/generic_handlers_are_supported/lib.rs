use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub fn path() -> PathBuf {
    todo!()
}

pub fn stream_file<T>(_inner: T) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::path), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>));
    bp
}
