use std::path::PathBuf;

use pavex::blueprint::{from, Blueprint};
use pavex::f;

pub struct Logger;

pub async fn extract_path(_inner: pavex::request::RequestHead) -> PathBuf {
    todo!()
}

pub async fn logger() -> Logger {
    todo!()
}

#[pavex::get(path = "/home")]
pub async fn stream_file(
    _inner: PathBuf,
    _logger: Logger,
    _http_client: &HttpClient,
) -> pavex::response::Response {
    todo!()
}

#[derive(Clone)]
#[pavex::prebuilt]
pub struct Config;

#[derive(Clone)]
pub struct HttpClient;

pub async fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.singleton(f!(crate::http_client));
    bp.request_scoped(f!(crate::extract_path));
    bp.transient(f!(crate::logger));
    bp.routes(from![crate]);
    bp
}
