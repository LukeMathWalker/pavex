use std::future::IntoFuture;
use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::{request::RequestHead, response::Response};

pub struct Logger;

pub fn extract_path(_inner: RequestHead) -> Result<PathBuf, ExtractPathError<String>> {
    todo!()
}

#[derive(Debug)]
pub struct ExtractPathError<T>(T);

pub fn handle_extract_path_error(
    _e: &ExtractPathError<String>,
    _logger: Logger,
) -> pavex::response::Response {
    todo!()
}

pub fn logger() -> Result<Logger, LoggerError> {
    todo!()
}

#[derive(Debug)]
pub struct LoggerError;

pub fn handle_logger_error(_e: &LoggerError) -> Response {
    todo!()
}

pub fn request_handler(
    _inner: PathBuf,
    _logger: Logger,
    _http_client: HttpClient,
) -> Result<Response, HandlerError> {
    todo!()
}

#[derive(Debug)]
pub struct HandlerError;

pub fn handle_handler_error(_e: &HandlerError) -> Response {
    todo!()
}

#[derive(Clone)]
pub struct Config;

pub fn config() -> Config {
    todo!()
}

#[derive(Clone)]
pub struct HttpClient;

#[derive(Debug, thiserror::Error)]
#[error("Http client error")]
pub struct HttpClientError;

pub fn http_client(_config: Config) -> Result<HttpClient, HttpClientError> {
    todo!()
}

#[derive(Debug)]
pub struct MiddlewareError;

pub fn handle_middleware_error(_e: &MiddlewareError) -> Response {
    todo!()
}

pub fn fallible_wrapping_middleware<T>(_next: Next<T>) -> Result<Response, MiddlewareError>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[derive(Debug)]
pub struct PPMiddlewareError;

pub fn handle_pp_middleware_error(_e: &PPMiddlewareError) -> Response {
    todo!()
}

pub fn fallible_pp_middleware<T>(_response: Response) -> Result<Response, PPMiddlewareError> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::http_client), Lifecycle::Singleton);
    bp.constructor(f!(crate::extract_path), Lifecycle::RequestScoped)
        .error_handler(f!(crate::handle_extract_path_error));
    bp.constructor(f!(crate::logger), Lifecycle::Transient)
        .error_handler(f!(crate::handle_logger_error));
    bp.wrap(f!(crate::fallible_wrapping_middleware))
        .error_handler(f!(crate::handle_middleware_error));
    bp.post_process(f!(crate::fallible_pp_middleware))
        .error_handler(f!(crate::handle_pp_middleware_error));
    bp.route(GET, "/home", f!(crate::request_handler))
        .error_handler(f!(crate::handle_handler_error));
    bp
}
