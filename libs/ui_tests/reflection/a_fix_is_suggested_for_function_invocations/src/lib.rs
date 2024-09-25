use pavex::blueprint::{reflection::RawIdentifiers, router::POST, Blueprint};
use pavex::f;

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(POST, "/home", f!(self::my_f()));
    bp
}
