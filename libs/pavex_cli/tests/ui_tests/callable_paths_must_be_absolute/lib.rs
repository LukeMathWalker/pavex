use pavex_builder::{f, AppBlueprint};

pub fn handler() -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new().route(f!(handler), "/home")
}
