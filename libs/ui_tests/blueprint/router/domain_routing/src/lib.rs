use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::request::RequestHead;
use pavex::response::Response;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("admin.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::admin_root));
        bp.fallback(f!(crate::admin_fallback));
        bp
    });
    bp.domain("company.com").nest({
        let mut bp = Blueprint::new();
        // Same path, different domain.
        bp.route(GET, "/", f!(crate::base_root));
        bp.route(GET, "/login", f!(crate::base_login));
        bp
    });
    bp.domain("ops.company.com").nest({
        let mut bp = Blueprint::new();
        bp.fallback(f!(crate::ops_fallback));
        bp
    });
    bp.domain("{sub}.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::base_sub));
        bp
    });
    bp.domain("{*any}.{sub}.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::base_any));
        bp
    });
    bp.fallback(f!(crate::root_fallback));
    bp
}

pub fn admin_root() -> pavex::response::Response {
    Response::ok().set_typed_body("admin.company.com /")
}

pub fn admin_fallback() -> pavex::response::Response {
    Response::ok().set_typed_body("admin.company.com fallback")
}

pub fn ops_fallback() -> pavex::response::Response {
    Response::ok().set_typed_body("ops.company.com fallback")
}

pub fn base_root() -> pavex::response::Response {
    Response::ok().set_typed_body("company.com /")
}

pub fn base_login() -> pavex::response::Response {
    Response::ok().set_typed_body("company.com /login")
}

pub fn base_sub() -> pavex::response::Response {
    Response::ok().set_typed_body("{sub}.company.com /")
}

pub fn base_any() -> pavex::response::Response {
    Response::ok().set_typed_body("{*any}.{sub}.company.com /")
}

pub fn root_fallback(head: &RequestHead) -> pavex::response::Response {
    let host = head
        .headers
        .get(pavex::http::header::HOST)
        .unwrap()
        .to_str()
        .unwrap();
    Response::ok().set_typed_body(format!("root fallback [{host}]"))
}
