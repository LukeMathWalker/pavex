use pavex::blueprint::{router::ANY, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(ANY, "/greet", f!(crate::routes::greet));
    bp
}
