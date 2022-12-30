use pavex_builder::{f, AppBlueprint};

pub fn stream_file<T>(_inner: T) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::stream_file::<std::path::PathBuf>), "/home");
    bp
}
