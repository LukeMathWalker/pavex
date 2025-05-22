use pavex::blueprint::{
    from,
    router::{ANY, GET},
    Blueprint,
};
use pavex::f;
use pavex::response::Response;

pub fn handler() -> Response {
    todo!()
}

#[pavex::get(path = "/{how}{many}{params}{can}{i}{chain}")]
pub fn too_many_params() -> Response {
    todo!()
}

#[pavex::get(path = "/{*invalid_catch_all}/hey")]
pub fn invalid_catch_all() -> Response {
    todo!()
}

#[pavex::get(path = "/room/{id}")]
pub fn room_id() -> Response {
    todo!()
}

#[pavex::get(path = "/room/{room_id}")]
pub fn conflicting_room_id() -> Response {
    todo!()
}

#[pavex::get(path = "/room/{}")]
pub fn unnamed() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);

    bp.prefix("/bp").nest({
        let mut bp = Blueprint::new();
        bp.route(ANY, "/{too}{many}{params}", f!(crate::handler));
        bp.route(GET, "/{*invalid_catch_all}/hey", f!(crate::handler));
        bp.route(GET, "/home/{id}", f!(crate::handler));
        // Route conflict with the previous one
        bp.route(GET, "/home/{home_id}", f!(crate::handler));
        // Unnamed parameter
        bp.route(GET, "/room/{}", f!(crate::handler));
        bp
    });

    bp
}
