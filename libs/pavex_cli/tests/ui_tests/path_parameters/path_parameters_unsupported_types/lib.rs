use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::http::StatusCode;
use pavex::request::path::PathParams;

pub fn tuple(params: PathParams<(u32, u32)>) -> StatusCode {
    todo!()
}

pub fn primitive(params: PathParams<u32>) -> StatusCode {
    todo!()
}

pub fn slice_ref(params: PathParams<&[u32]>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct MyStruct {
    x: u32,
    y: u32,
}

pub fn reference<T>(params: PathParams<&T>) -> StatusCode {
    todo!()
}

#[PathParams]
pub enum MyEnum {
    A(u32),
    B,
    C { x: u32, y: u32 },
}

pub fn enum_(params: PathParams<MyEnum>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct UnitStruct;

pub fn unit_struct(params: PathParams<UnitStruct>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct TupleStruct(u32, u32);

pub fn tuple_struct(params: PathParams<TupleStruct>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex::request::path::PathParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::request::path::errors::ExtractPathParamsError::into_response
    ));
    bp.route(GET, "/a/:x", f!(crate::primitive));
    bp.route(GET, "/b/:x/:y", f!(crate::tuple));
    bp.route(GET, "/c/:x/:z", f!(crate::slice_ref));
    bp.route(GET, "/d/:x/:y", f!(crate::reference::<crate::MyStruct>));
    bp.route(GET, "/e/:x/:y", f!(crate::enum_));
    bp.route(GET, "/f/:x/:y", f!(crate::tuple_struct));
    bp.route(GET, "/g/:x/:y", f!(crate::unit_struct));
    bp
}
