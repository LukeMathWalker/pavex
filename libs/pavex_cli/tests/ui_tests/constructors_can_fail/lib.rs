use std::path::PathBuf;

use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Logger;

pub fn extract_path(
    _inner: pavex_runtime::http::Request<pavex_runtime::hyper::body::Body>,
) -> Result<PathBuf, ExtractPathError<String>> {
    todo!()
}

pub struct ExtractPathError<T>(T);

pub fn handle_extract_path_error(
    _e: &ExtractPathError<String>,
    _logger: Logger,
) -> pavex_runtime::http::Response<pavex_runtime::hyper::body::Body> {
    todo!()
}

pub fn logger() -> Result<Logger, LoggerError> {
    todo!()
}

pub struct LoggerError;

pub fn handle_logger_error(
    _e: &LoggerError,
) -> pavex_runtime::http::Response<pavex_runtime::hyper::body::Body> {
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

pub struct HttpClientError;

pub fn http_client(_config: Config) -> Result<HttpClient, HttpClientError> {
    todo!()
}

pub fn handle_http_client_error(
    _e: &HttpClientError,
) -> pavex_runtime::http::Response<pavex_runtime::hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::http_client), Lifecycle::Singleton)
        .error_handler(f!(crate::handle_http_client_error));
    bp.constructor(f!(crate::extract_path), Lifecycle::RequestScoped)
        .error_handler(f!(crate::handle_extract_path_error));
    bp.constructor(f!(crate::logger), Lifecycle::Transient)
        .error_handler(f!(crate::handle_logger_error));
    bp.route(f!(crate::stream_file), "/home");
    bp
}
