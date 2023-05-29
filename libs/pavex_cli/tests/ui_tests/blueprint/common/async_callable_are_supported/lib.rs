use std::path::PathBuf;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub struct Logger;

pub async fn extract_path(_inner: pavex_runtime::request::RequestHead) -> PathBuf {
    todo!()
}

pub async fn logger() -> Logger {
    todo!()
}

pub async fn stream_file(
    _inner: PathBuf,
    _logger: Logger,
    _http_client: HttpClient,
) -> pavex_runtime::response::Response {
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

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::http_client), Lifecycle::Singleton);
    bp.constructor(f!(crate::extract_path), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::logger), Lifecycle::Transient);
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
