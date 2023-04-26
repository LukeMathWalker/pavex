use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn header1() -> http_01::header::HeaderName {
    todo!()
}

pub fn header2() -> http_02::header::HeaderName {
    todo!()
}

pub fn handler(
    _h1: http_01::header::HeaderName,
    _h2: http_02::header::HeaderName,
) -> pavex_runtime::response::Response {
    todo!()
}

pub fn dep_blueprint(bp: &mut Blueprint) {
    bp.constructor(f!(crate::header1), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::header2), Lifecycle::RequestScoped);
    bp.route(GET, "/handler", f!(crate::handler));
}
