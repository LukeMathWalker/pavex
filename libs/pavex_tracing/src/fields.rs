//! Utilities to log common resources with consistent naming and representations.
//!
//! For well-known resources, this module exposes:
//!
//! - A constant holding the conventional field name used when logging that resource
//! - A function to compute the conventional log representation of that resource in the context
//!   of an HTTP API
//!
//! For example, you have [`HTTP_REQUEST_METHOD`] and [`http_request_method`] for the
//! `http.request.method` field.  
//!
//! The naming follows [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/)
//! whenever possible.
//!
//! # Exhaustiveness
//!
//! The module doesn't cover the entirety of OpenTelemetry's semantic convention specification.  
//! Feel free to open a PR if you need a **stable** field that isn't currently covered!

use pavex::http::{Method, Version};
use pavex::request::path::MatchedPathPattern;
use pavex::request::RequestHead;
use pavex::response::Response;
use pavex::telemetry::ServerRequestId;
use tracing::Value;

/// The field name for the HTTP method of the incoming request (if canonical),
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/http/).
///
/// Use [`http_request_method`] to populate the field.
pub const HTTP_REQUEST_METHOD: &'static str = "http.request.method";

/// The field name to record the server-generated identifier for this request.  
/// This field doesn't appear in OpenTelemetry's semantic convention specification.
///
/// Use [`http_request_server_id`] to populate the field.
pub const HTTP_REQUEST_SERVER_ID: &'static str = "http.request.server_id";

/// The field name to record the `Display` representation of an error.  
/// This field doesn't appear in OpenTelemetry's semantic convention specification.
///
/// Use [`error_message`] to populate the field.
pub const ERROR_MESSAGE: &'static str = "error.message";

/// The field name to record the `Debug` representation of an error.  
/// This field doesn't appear in OpenTelemetry's semantic convention specification.
///
/// Use [`error_message`] to populate the field.
pub const ERROR_DETAILS: &'static str = "error.details";

/// The field name to record the chain of sources for an error.  
/// This field doesn't appear in OpenTelemetry's semantic convention specification.
///
/// Use [`error_source_chain`] to populate the field.
pub const ERROR_SOURCE_CHAIN: &'static str = "error.source_chain";

/// The field name for the HTTP status code of the outgoing response,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/http/).
///
/// Use [`http_response_status_code`] to populate the field.
pub const HTTP_RESPONSE_STATUS_CODE: &'static str = "http.response.status_code";

/// The field name for path pattern matched by the incoming request,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/http/).
///
/// Use [`http_route`] to populate the field.
pub const HTTP_ROUTE: &'static str = "http.route";

/// The name of the network protocol used by the incoming request,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/network/).
///
/// Use [`network_protocol_name`] to populate the field.
pub const NETWORK_PROTOCOL_NAME: &'static str = "network.protocol.name";

/// The version of the network protocol used by the incoming request,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/network/).
///
/// Use [`network_protocol_version`] to populate the field.
pub const NETWORK_PROTOCOL_VERSION: &'static str = "network.protocol.version";

/// The path targeted by the incoming request,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/url/).
///
/// Use [`url_path`] to populate the field.
pub const URL_PATH: &'static str = "url.path";

/// The query string of the incoming request,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/url/).
///
/// Use [`url_query`] to populate the field.
pub const URL_QUERY: &'static str = "url.query";

/// The user agent header for the incoming request,
/// according to [OpenTelemetry's semantic convention](https://opentelemetry.io/docs/specs/semconv/attributes-registry/user-agent/).
///
/// Use [`user_agent_original`] to populate the field.
pub const USER_AGENT_ORIGINAL: &'static str = "user_agent.original";

/// The canonical representation for the value in [`HTTP_REQUEST_METHOD`].
///
/// If the HTTP method is not canonical, it is set to `_OTHER`.
pub fn http_request_method(request_head: &RequestHead) -> impl Value {
    match request_head.method {
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::TRACE => "TRACE",
        Method::PATCH => "PATCH",
        Method::CONNECT => "CONNECT",
        Method::HEAD => "HEAD",
        Method::DELETE => "DELETE",
        Method::OPTIONS => "OPTIONS",
        _ => "_OTHER",
    }
}

/// The canonical representation for the value in [`HTTP_REQUEST_SERVER_ID`].
pub fn http_request_server_id(id: ServerRequestId) -> impl Value {
    tracing::field::display(id)
}

/// The canonical representation for the value in [`HTTP_RESPONSE_STATUS_CODE`].
pub fn http_response_status_code(response: &Response) -> impl Value {
    response.status().as_u16()
}

/// The canonical representation for the value in [`HTTP_ROUTE`].
pub fn http_route(matched_path_pattern: MatchedPathPattern) -> impl Value {
    tracing::field::display(matched_path_pattern)
}

/// The canonical representation for the value in [`NETWORK_PROTOCOL_NAME`].
pub fn network_protocol_name() -> impl Value {
    "http"
}

/// The canonical representation for the value in [`NETWORK_PROTOCOL_VERSION`].
pub fn network_protocol_version(request_head: &RequestHead) -> impl Value {
    match request_head.version {
        Version::HTTP_09 => "0.9",
        Version::HTTP_10 => "1.0",
        Version::HTTP_11 => "1.1",
        Version::HTTP_2 => "2.0",
        Version::HTTP_3 => "3.0",
        _ => "_OTHER",
    }
}

/// The canonical representation for the value in [`URL_PATH`].
pub fn url_path(request_head: &RequestHead) -> impl Value + '_ {
    request_head.target.path()
}

/// The canonical representation for the value in [`URL_QUERY`].
pub fn url_query(request_head: &RequestHead) -> impl Value + '_ {
    request_head.target.query().unwrap_or_default()
}

/// The canonical representation for the value in [`USER_AGENT_ORIGINAL`].
pub fn user_agent_original(request_head: &RequestHead) -> impl Value + '_ {
    request_head
        .headers
        .get("User-Agent")
        .map(|h| h.to_str().unwrap_or_default())
        .unwrap_or_default()
}

/// The canonical representation for the value in [`ERROR_MESSAGE`].
pub fn error_message<E: std::fmt::Display>(e: E) -> impl Value {
    tracing::field::display(e)
}

/// The canonical representation for the value in [`ERROR_DETAILS`].
pub fn error_details<E: std::fmt::Debug>(e: E) -> impl Value {
    tracing::field::debug(e)
}

/// The canonical representation for the value in [`ERROR_SOURCE_CHAIN`].
pub fn error_source_chain<E: std::error::Error>(e: E) -> impl Value {
    _error_source_chain(e)
}

fn _error_source_chain<E: std::error::Error>(e: E) -> String {
    use std::fmt::Write as _;

    let mut chain = String::new();
    let mut source = e.source();
    while let Some(s) = source {
        let _ = writeln!(chain, "- {}", s);
        source = s.source();
    }
    chain
}
