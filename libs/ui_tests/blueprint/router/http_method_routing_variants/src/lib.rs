use std::str::FromStr;

use pavex::blueprint::{
    from,
    router::{
        MethodGuard, ANY, ANY_WITH_EXTENSIONS, CONNECT, DELETE, GET, HEAD, OPTIONS, PATCH, POST,
        PUT, TRACE,
    },
    Blueprint,
};
use pavex::f;
use pavex::http::Method;

pub fn handler() -> pavex::response::Response {
    todo!()
}

#[pavex::delete(path = "/delete")]
pub fn delete() -> pavex::response::Response {
    todo!()
}

#[pavex::get(path = "/get")]
pub fn get() -> pavex::response::Response {
    todo!()
}

#[pavex::head(path = "/head")]
pub fn head() -> pavex::response::Response {
    todo!()
}

#[pavex::options(path = "/options")]
pub fn options() -> pavex::response::Response {
    todo!()
}

#[pavex::patch(path = "/patch")]
pub fn patch() -> pavex::response::Response {
    todo!()
}

#[pavex::post(path = "/post")]
pub fn post() -> pavex::response::Response {
    todo!()
}

#[pavex::put(path = "/put")]
pub fn put() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = "TRACE", path = "/trace")]
pub fn trace() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = "CONNECT", path = "/connect")]
pub fn connect() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = ["PATCH", "POST"], path = "/mixed")]
pub fn mixed() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = "CUSTOM", path = "/custom", allow(non_standard_methods))]
pub fn custom() -> pavex::response::Response {
    todo!()
}

#[pavex::route(method = ["CUSTOM", "HEY"], path = "/mixed_custom", allow(non_standard_methods))]
pub fn mixed_custom() -> pavex::response::Response {
    todo!()
}

#[pavex::route(path = "/any", allow(any_method))]
pub fn any() -> pavex::response::Response {
    todo!()
}

#[pavex::route(path = "/any_with_extensions", allow(any_method, non_standard_methods))]
pub fn any_with_extensions() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);

    bp.prefix("/bp").nest({
        let mut bp = Blueprint::new();
        bp.route(CONNECT, "/connect", f!(crate::handler));
        bp.route(DELETE, "/delete", f!(crate::handler));
        bp.route(GET, "/get", f!(crate::handler));
        bp.route(HEAD, "/head", f!(crate::handler));
        bp.route(OPTIONS, "/options", f!(crate::handler));
        bp.route(PATCH, "/patch", f!(crate::handler));
        bp.route(POST, "/post", f!(crate::handler));
        bp.route(PUT, "/put", f!(crate::handler));
        bp.route(TRACE, "/trace", f!(crate::handler));
        bp.route(ANY, "/any", f!(crate::handler));
        bp.route(ANY_WITH_EXTENSIONS, "/any_w_extensions", f!(crate::handler));
        bp.route(PATCH.or(POST), "/mixed", f!(crate::handler));
        let custom_method: MethodGuard = Method::from_str("CUSTOM").unwrap().into();
        let custom2_method: MethodGuard = Method::from_str("HEY").unwrap().into();
        bp.route(custom_method.clone(), "/custom", f!(crate::handler));
        bp.route(
            custom_method.or(custom2_method).or(GET),
            "/mixed_with_custom",
            f!(crate::handler),
        );
        bp
    });
    bp
}
