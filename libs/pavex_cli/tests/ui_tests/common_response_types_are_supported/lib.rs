use std::borrow::Cow;

use bytes::{Bytes, BytesMut};
use http::response::Parts;
use http::status::StatusCode;
use http_body::{Empty, Full};
use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn response() -> pavex_runtime::response::Response {
    todo!()
}

pub fn static_str() -> &'static str {
    todo!()
}

pub fn static_u8_slice() -> &'static [u8] {
    todo!()
}

pub fn string() -> String {
    todo!()
}

pub fn vec_u8() -> Vec<u8> {
    todo!()
}

pub fn bytes() -> Bytes {
    todo!()
}

pub fn bytes_mut() -> BytesMut {
    todo!()
}

pub fn empty() -> Empty<Bytes> {
    todo!()
}

pub fn status_code() -> StatusCode {
    todo!()
}

pub fn parts() -> Parts {
    todo!()
}

pub fn full() -> Full<Bytes> {
    todo!()
}

pub fn cow_static_str() -> Cow<'static, str> {
    todo!()
}

pub fn cow_static_u8_slice() -> Cow<'static, [u8]> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/response", f!(crate::response));
    bp.route(GET, "/static_str", f!(crate::static_str));
    bp.route(GET, "/string", f!(crate::string));
    bp.route(GET, "/vec_u8", f!(crate::vec_u8));
    bp.route(GET, "/cow_static_str", f!(crate::cow_static_str));
    bp.route(GET, "/bytes", f!(crate::bytes));
    bp.route(GET, "/bytes_mut", f!(crate::bytes_mut));
    bp.route(GET, "/empty", f!(crate::empty));
    bp.route(GET, "/status_code", f!(crate::status_code));
    bp.route(GET, "/parts", f!(crate::parts));
    bp.route(GET, "/full", f!(crate::full));
    bp.route(GET, "/static_u8_slice", f!(crate::static_u8_slice));
    bp.route(GET, "/cow_static_u8_slice", f!(crate::cow_static_u8_slice));
    bp
}
