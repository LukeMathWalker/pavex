use pavex::blueprint::{from, Blueprint};
use pavex::http::StatusCode;
use pavex::request::path::PathParams;

#[pavex::get(path = "/b/{x}/{y}")]
pub fn tuple(_params: PathParams<(u32, u32)>) -> StatusCode {
    todo!()
}

#[pavex::get(path = "/a/{x}")]
pub fn primitive(_params: PathParams<u32>) -> StatusCode {
    todo!()
}

#[pavex::get(path = "/c/{x}/{z}")]
pub fn slice_ref(_params: PathParams<&[u32]>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct MyStruct {
    x: u32,
    y: u32,
}

#[pavex::get(path = "/d/{x}/{y}")]
pub fn reference(_params: PathParams<&MyStruct>) -> StatusCode {
    todo!()
}

#[PathParams]
pub enum MyEnum {
    A(u32),
    B,
    C { x: u32, y: u32 },
}

#[pavex::get(path = "/e/{x}/{y}")]
pub fn enum_(_params: PathParams<MyEnum>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct UnitStruct;

#[pavex::get(path = "/g/{x}/{y}")]
pub fn unit_struct(_params: PathParams<UnitStruct>) -> StatusCode {
    todo!()
}

#[PathParams]
pub struct TupleStruct(u32, u32);

#[pavex::get(path = "/f/{x}/{y}")]
pub fn tuple_struct(_params: PathParams<TupleStruct>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]);
    bp.routes(from![crate]);
    bp
}
