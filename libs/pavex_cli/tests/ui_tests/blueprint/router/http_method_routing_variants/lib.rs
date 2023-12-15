use std::str::FromStr;

use pavex::blueprint::{
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

pub fn blueprint() -> Blueprint {
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
}
