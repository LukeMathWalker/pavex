use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::{f, t};

pub struct Logger;

pub async fn extract_path(_inner: pavex::request::RequestHead) -> PathBuf {
    todo!()
}

pub async fn logger() -> Logger {
    todo!()
}

pub async fn stream_file(
    _inner: PathBuf,
    _logger: Logger,
    _http_client: &HttpClient,
) -> pavex::response::Response {
    todo!()
}

#[derive(Clone)]
pub struct Config;

#[derive(Clone)]
pub struct HttpClient;

pub async fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::http_client));
    bp.request_scoped(f!(crate::extract_path));
    bp.transient(f!(crate::logger));
    bp.prebuilt(t!(crate::Config));
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
