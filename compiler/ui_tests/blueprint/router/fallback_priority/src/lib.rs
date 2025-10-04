use pavex::Response;
use pavex::Blueprint;

#[pavex::get(path = "/")]
pub fn root() -> Response {
    todo!()
}

#[pavex::get(path = "/id")]
pub fn id() -> Response {
    todo!()
}

#[pavex::fallback]
pub fn unauthorized() -> Response {
    Response::unauthorized()
}

#[pavex::fallback]
pub fn forbidden() -> Response {
    Response::forbidden()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prefix("/users").nest({
        let mut bp = Blueprint::new();
        bp.route(ROOT);
        bp.nest({
            let mut bp = Blueprint::new();
            bp.route(ID);
            bp.fallback(FORBIDDEN);
            bp
        });
        bp.fallback(UNAUTHORIZED);
        bp
    });
    bp
}
