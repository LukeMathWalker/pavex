use pavex_builder::{f, AppBlueprint};

pub fn handler() -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(handler), "/home");
    bp
}
