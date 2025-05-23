use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub fn handler() -> pavex::response::Response {
    todo!()
}

#[pavex::fallback]
pub fn unauthorized() -> pavex::response::Response {
    Response::unauthorized()
}

#[pavex::fallback]
pub fn forbidden() -> pavex::response::Response {
    Response::forbidden()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/users").nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::handler));
        bp.nest({
            let mut bp = Blueprint::new();
            bp.route(GET, "/id", f!(crate::handler));
            bp.fallback(FORBIDDEN);
            bp
        });
        bp.fallback(UNAUTHORIZED);
        bp
    });
    bp
}
