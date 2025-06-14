use pavex::blueprint::{
    router::{GET, POST},
    Blueprint,
};
use pavex::f;

pub fn handler() -> pavex::response::Response {
    todo!()
}

#[pavex::fallback]
pub fn fallback1() -> pavex::response::Response {
    todo!()
}

#[pavex::fallback]
pub fn fallback2() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest({
        let mut bp = Blueprint::new();
        bp.route(GET, "/id", f!(crate::handler));
        bp.fallback(FALLBACK_1);
        bp
    });
    bp.nest({
        let mut bp = Blueprint::new();
        bp.route(POST, "/id", f!(crate::handler));
        bp.fallback(FALLBACK_2);
        bp
    });
    bp
}
