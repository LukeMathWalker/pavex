use pavex_builder::{f, router::GET, Blueprint, Lifecycle};
use pavex_runtime::extract::route::RouteParams;

pub fn tuple(params: RouteParams<(u32, u32)>) -> String {
    todo!()
}

pub fn primitive(params: RouteParams<u32>) -> String {
    todo!()
}

pub fn slice_ref(params: RouteParams<&[u32]>) -> String {
    todo!()
}

pub struct MyStruct {
    x: u32,
    y: u32,
}

pub fn reference(params: RouteParams<&MyStruct>) -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex_runtime::extract::route::RouteParams::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::route::errors::ExtractRouteParamsError::into_response
    ));
    bp.route(GET, "a/:x", f!(crate::primitive));
    bp.route(GET, "b/:x/:y", f!(crate::tuple));
    bp.route(GET, "c/:x/:z", f!(crate::slice_ref));
    bp.route(GET, "d/:x/:y", f!(crate::reference));
    bp
}
