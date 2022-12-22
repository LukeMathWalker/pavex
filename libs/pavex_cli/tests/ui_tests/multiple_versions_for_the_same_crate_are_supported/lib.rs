use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn header1() -> http_01::header::HeaderName {
    todo!()
}

pub fn header2() -> http_02::header::HeaderName {
    todo!()
}

pub fn stream_file(
    _h1: http_01::header::HeaderName,
    _h2: http_02::header::HeaderName,
) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::header1), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::header2), Lifecycle::RequestScoped);
    bp.route(f!(crate::stream_file), "/home");
    bp
}
