use pavex_builder::{f, AppBlueprint};

pub fn stream_file(input: (usize, isize)) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.route(f!(crate::stream_file), "/home");
    bp
}
