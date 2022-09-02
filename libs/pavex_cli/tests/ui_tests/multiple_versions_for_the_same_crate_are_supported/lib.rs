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
) -> http_02::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::header1), Lifecycle::RequestScoped)
        .constructor(f!(crate::header2), Lifecycle::RequestScoped)
        .route(f!(crate::stream_file), "/home")
}
