use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Streamer;

impl Streamer {
    pub fn stream_file(_logger: dep::Logger) -> http::Response<hyper::body::Body> {
        todo!()
    }
}

pub fn blueprint() -> AppBlueprint {
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(dep::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(::dep::new_logger), Lifecycle::RequestScoped);
    bp.route(f!(crate::Streamer::stream_file), "/home");
    bp
}
