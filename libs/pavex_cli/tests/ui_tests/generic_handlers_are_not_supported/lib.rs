use pavex_builder::{f, AppBlueprint};

pub fn stream_file<T>(_inner: T) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route(f!(crate::stream_file::<std::path::PathBuf>), "/home")
}
