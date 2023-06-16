use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::extract::route::RouteParams;
use pavex::f;
use pavex::http::StatusCode;

pub fn tuple(params: RouteParams<(u32, u32)>) -> StatusCode {
    todo!()
}

pub fn primitive(params: RouteParams<u32>) -> StatusCode {
    todo!()
}

pub fn slice_ref(params: RouteParams<&[u32]>) -> StatusCode {
    todo!()
}

#[RouteParams]
pub struct MyStruct {
    x: u32,
    y: u32,
}

pub fn reference<T>(params: RouteParams<&T>) -> StatusCode {
    todo!()
}

#[RouteParams]
pub enum MyEnum {
    A(u32),
    B,
    C { x: u32, y: u32 },
}

pub fn enum_(params: RouteParams<MyEnum>) -> StatusCode {
    todo!()
}

#[RouteParams]
pub struct UnitStruct;

pub fn unit_struct(params: RouteParams<UnitStruct>) -> StatusCode {
    todo!()
}

#[RouteParams]
pub struct TupleStruct(u32, u32);

pub fn tuple_struct(params: RouteParams<TupleStruct>) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex::extract::route::errors::ExtractRouteParamsError::into_response
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
