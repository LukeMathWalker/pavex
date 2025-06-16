use std::path::PathBuf;

use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

pub struct Logger;

#[pavex::request_scoped]
pub async fn extract_path(
    _inner: pavex::request::RequestHead,
) -> Result<PathBuf, ExtractPathError<String>> {
    todo!()
}

pub struct ExtractPathError<T>(T);

#[pavex::error_handler]
pub fn handle_extract_path_error(#[px(error_ref)] _e: &ExtractPathError<String>, _logger: Logger) -> Response {
    todo!()
}

#[pavex::transient]
pub fn logger() -> Logger {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn stream_file(_inner: PathBuf, _logger: Logger, _http_client: HttpClient) -> Response {
    todo!()
}

#[derive(Clone)]
#[pavex::prebuilt(clone_if_necessary)]
pub struct Config;

#[derive(Clone)]
pub struct HttpClient;

#[pavex::singleton(clone_if_necessary)]
pub fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
