use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

pub fn handler() -> String {
    todo!()
}

#[pavex::get(path = "api")]
pub fn missing_leading_slash() -> String {
    todo!()
}

#[pavex::get(path = "")]
pub fn empty_path() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp.prefix("/bp").nest({
        let mut bp = Blueprint::new();
        // Empty path is accepted
        bp.route(GET, "", f!(crate::handler));
        // If the path is not empty, it *must* start with a `/`
        bp.route(GET, "api", f!(crate::handler));
        bp
    });
    bp
}
