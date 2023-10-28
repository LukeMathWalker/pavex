use pavex::blueprint::{
    router::{ANY, GET},
    Blueprint,
};
use pavex::f;

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(ANY, "/:too:many:params", f!(crate::handler));
    bp.route(GET, "/*invalid_catch_all/hey", f!(crate::handler));
    bp.route(GET, "/home/:id", f!(crate::handler));
    // Route conflict with the previous one
    bp.route(GET, "/home/:home_id", f!(crate::handler));
    // Unnamed parameter
    bp.route(GET, "/room/:", f!(crate::handler));
    bp
}
