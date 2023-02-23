use std::path::PathBuf;

use pavex_builder::router::GET;
use pavex_builder::{f, Blueprint, Lifecycle};

pub struct Logger;

pub fn extract_path(_inner: pavex_runtime::http::Request<pavex_runtime::hyper::Body>) -> PathBuf {
    todo!()
}

pub fn logger() -> Logger {
    todo!()
}

pub fn stream_file(
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

pub fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn app_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::http_client), Lifecycle::Singleton);
    bp.constructor(f!(crate::extract_path), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::logger), Lifecycle::Transient);
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
