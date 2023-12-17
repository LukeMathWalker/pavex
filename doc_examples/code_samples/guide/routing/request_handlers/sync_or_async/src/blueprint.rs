use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/async_greet", f!(crate::routes::async_greet));
    bp.route(GET, "/sync_greet", f!(crate::routes::sync_greet));
    bp
}
