use pavex::blueprint::{
    router::{GET, POST},
    Blueprint,
};
use pavex::f;
use pavex::response::Response;

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn unauthorized() -> pavex::response::Response {
    Response::unauthorized()
}

pub fn forbidden() -> pavex::response::Response {
    Response::forbidden()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest_at("/users", {
        let mut bp = Blueprint::new();
        bp.route(GET, "/", f!(crate::handler));
        bp.nest({
            let mut bp = Blueprint::new();
            bp.route(GET, "/id", f!(crate::handler));
            bp.fallback(f!(crate::forbidden));
            bp
        });
        bp.fallback(f!(crate::unauthorized));
        bp
    });
    bp
}
