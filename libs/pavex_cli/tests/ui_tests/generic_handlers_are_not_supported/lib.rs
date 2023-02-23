use pavex_builder::{f, router::GET, Blueprint};

pub fn stream_file<T>(_inner: T) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>));
    bp
}
