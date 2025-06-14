use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::{http::StatusCode, request::path::PathParams};

#[PathParams]
pub struct MissingOne {
    x: u32,
    y: u32,
}

#[pavex::get(path = "/a/{x}")]
pub fn missing_one(_params: PathParams<MissingOne>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct MissingTwo {
    x: u32,
    y: u32,
    z: u32,
}

#[pavex::get(path = "/b/{x}")]
pub fn missing_two(_params: PathParams<MissingTwo>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct NoPathParams {
    x: u32,
    y: u32,
}

#[pavex::get(path = "/c")]
pub fn no_path_params(_params: PathParams<NoPathParams>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(pavex::request::path::PathParams::extract))
        .error_handler(f!(
            pavex::request::path::errors::ExtractPathParamsError::into_response
        ));
    bp.routes(from![crate]);
    bp
}
