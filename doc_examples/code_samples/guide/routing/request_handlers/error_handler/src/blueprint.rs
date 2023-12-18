use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/greet", f!(crate::routes::greet))
        .error_handler(f!(crate::error_handler::greet_error_handler));
    bp
}
