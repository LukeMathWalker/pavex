use pavex_builder::{
    f,
    router::{MethodGuard, ANY, CONNECT, DELETE, GET, HEAD, OPTIONS, PATCH, POST, PUT, TRACE},
    Blueprint,
};

pub fn handler() -> pavex_runtime::response::Response {
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
    bp.route(
        MethodGuard::new([
            pavex_runtime::http::Method::PATCH,
            pavex_runtime::http::Method::POST,
        ]),
        "/mixed",
        f!(crate::handler),
    );
    bp
}
