use pavex_builder::{f, Blueprint, Lifecycle};

pub struct Streamer;

impl Streamer {
    pub fn stream_file(_logger: dep::Logger) -> pavex_runtime::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(dep::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(::dep::new_logger), Lifecycle::RequestScoped);
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
