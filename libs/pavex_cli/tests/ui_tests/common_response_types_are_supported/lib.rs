use std::borrow::Cow;

use bytes::{Bytes, BytesMut};
use http::response::Parts;
use http::status::StatusCode;
use http_body::{Empty, Full};
use pavex_builder::{f, Blueprint, Lifecycle};

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
    bp.route(f!(crate::response), "/response");
    bp.route(f!(crate::static_str), "/static_str");
    bp.route(f!(crate::string), "/string");
    bp.route(f!(crate::vec_u8), "/vec_u8");
    bp.route(f!(crate::cow_static_str), "/cow_static_str");
    bp.route(f!(crate::bytes), "/bytes");
    bp.route(f!(crate::bytes_mut), "/bytes_mut");
    bp.route(f!(crate::empty), "/empty");
    bp.route(f!(crate::status_code), "/status_code");
    bp.route(f!(crate::parts), "/parts");
    bp.route(f!(crate::full), "/full");
    bp.route(f!(crate::static_u8_slice), "/static_u8_slice");
    bp.route(f!(crate::cow_static_u8_slice), "/cow_static_u8_slice");
    bp
}
