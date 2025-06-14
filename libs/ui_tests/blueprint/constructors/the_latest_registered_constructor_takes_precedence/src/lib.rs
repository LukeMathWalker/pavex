use pavex::blueprint::{constructor::Lifecycle, from, Blueprint};
use pavex::f;

pub struct Streamer;

#[pavex::methods]
impl Streamer {
    #[pavex::get(path = "/home")]
    pub fn stream_file(_logger: dep_55dca802::Logger) -> pavex::response::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(dep_55dca802::new_logger), Lifecycle::Singleton);
    bp.constructor(f!(::dep_55dca802::new_logger), Lifecycle::RequestScoped);
    bp.routes(from![crate]);
    bp
}
