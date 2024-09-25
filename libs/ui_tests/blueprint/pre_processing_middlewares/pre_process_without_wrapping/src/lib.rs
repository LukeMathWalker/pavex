use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Processing;
use pavex::response::Response;

pub fn pre() -> Processing {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.pre_process(f!(crate::pre));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
