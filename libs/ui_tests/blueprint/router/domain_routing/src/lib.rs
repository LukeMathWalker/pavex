use pavex::request::RequestHead;
use pavex::response::Response;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("admin.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(ADMIN_ROOT);
        bp.fallback(ADMIN_FALLBACK);
        bp
    });
    bp.domain("company.com").nest({
        let mut bp = Blueprint::new();
        // Same path, different domain.
        bp.route(BASE_ROOT);
        bp.route(BASE_LOGIN);
        bp
    });
    bp.domain("ops.company.com").nest({
        let mut bp = Blueprint::new();
        bp.fallback(OPS_FALLBACK);
        bp
    });
    bp.domain("{sub}.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(BASE_SUB);
        bp
    });
    bp.domain("{*any}.{sub}.company.com").nest({
        let mut bp = Blueprint::new();
        bp.route(BASE_ANY);
        bp
    });
    bp.fallback(ROOT_FALLBACK);
    bp
}

#[pavex::get(path = "/")]
pub fn admin_root() -> pavex::response::Response {
    Response::ok().set_typed_body("admin.company.com /")
}

#[pavex::fallback]
pub fn admin_fallback() -> pavex::response::Response {
    Response::ok().set_typed_body("admin.company.com fallback")
}

#[pavex::fallback]
pub fn ops_fallback() -> pavex::response::Response {
    Response::ok().set_typed_body("ops.company.com fallback")
}

#[pavex::get(path = "/")]
pub fn base_root() -> pavex::response::Response {
    Response::ok().set_typed_body("company.com /")
}

#[pavex::get(path = "/login")]
pub fn base_login() -> pavex::response::Response {
    Response::ok().set_typed_body("company.com /login")
}

#[pavex::get(path = "/")]
pub fn base_sub() -> pavex::response::Response {
    Response::ok().set_typed_body("{sub}.company.com /")
}

#[pavex::get(path = "/")]
pub fn base_any() -> pavex::response::Response {
    Response::ok().set_typed_body("{*any}.{sub}.company.com /")
}

#[pavex::fallback]
pub fn root_fallback(head: &RequestHead) -> pavex::response::Response {
    let host = head
        .headers
        .get(pavex::http::header::HOST)
        .unwrap()
        .to_str()
        .unwrap();
    Response::ok().set_typed_body(format!("root fallback [{host}]"))
}
