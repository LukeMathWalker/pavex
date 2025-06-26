use std::path::PathBuf;

use pavex::{blueprint::from, Blueprint};

pub struct Logger;

#[pavex::request_scoped]
pub async fn extract_path(_inner: pavex::request::RequestHead) -> PathBuf {
    todo!()
}

#[pavex::transient]
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

#[pavex::singleton]
pub async fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
