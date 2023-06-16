use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

#[derive(Clone)]
pub struct Streamer {
    pub a: usize,
    pub b: isize,
}

pub fn streamer() -> Streamer {
    todo!()
}

pub fn stream_file(Streamer { a, b }: Streamer) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::streamer), Lifecycle::Singleton);
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
