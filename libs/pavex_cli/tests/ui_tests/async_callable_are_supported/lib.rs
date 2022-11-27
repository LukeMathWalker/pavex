use pavex_builder::{f, AppBlueprint, Lifecycle};
use std::path::PathBuf;

pub struct Logger;

pub async fn extract_path(
    _inner: pavex_runtime::http::Request<pavex_runtime::hyper::body::Body>,
) -> PathBuf {
    todo!()
}

pub async fn logger() -> Logger {
    todo!()
}

pub async fn stream_file(
    _inner: PathBuf,
    _logger: Logger,
    _http_client: HttpClient,
) -> pavex_runtime::http::Response<pavex_runtime::hyper::body::Body> {
    todo!()
}

pub struct Config;

pub fn config() -> Config {
    todo!()
}

#[derive(Clone)]
pub struct HttpClient;

pub async fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::http_client), Lifecycle::Singleton)
        .constructor(f!(crate::extract_path), Lifecycle::RequestScoped)
        .constructor(f!(crate::logger), Lifecycle::Transient)
        .route(f!(crate::stream_file), "/home")
}
