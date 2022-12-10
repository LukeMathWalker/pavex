use pavex_builder::{f, AppBlueprint, Lifecycle};
use std::path::PathBuf;

pub struct Logger;

pub fn extract_path(
    _inner: pavex_runtime::http::Request<pavex_runtime::hyper::body::Body>,
) -> PathBuf {
    todo!()
}

pub fn logger() -> Logger {
    todo!()
}

pub fn stream_file(
    _inner: PathBuf,
    _logger: Logger,
    _http_client: HttpClient,
) -> pavex_runtime::http::Response<pavex_runtime::hyper::body::Body> {
    todo!()
}

#[derive(Clone)]
pub struct Config;

pub fn config() -> Config {
    todo!()
}

#[derive(Clone)]
pub struct HttpClient;

pub fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::http_client), Lifecycle::Singleton)
        .constructor(f!(crate::extract_path), Lifecycle::RequestScoped)
        .constructor(f!(crate::logger), Lifecycle::Transient)
        .route(f!(crate::stream_file), "/home")
}
