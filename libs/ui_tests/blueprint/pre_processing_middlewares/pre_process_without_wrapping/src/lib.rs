use pavex::middleware::Processing;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::pre_process]
pub fn pre() -> Processing {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.pre_process(PRE);
    bp.routes(from![crate]);
    bp
}
