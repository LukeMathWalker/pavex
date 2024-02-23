use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(super::length::<crate::input::GreetBody>),
        Lifecycle::RequestScoped,
    );
    bp.constructor(f!(super::json), Lifecycle::RequestScoped);
    bp
}
