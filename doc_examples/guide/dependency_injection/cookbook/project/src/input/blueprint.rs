use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(crate::input::length::<crate::input::GreetBody>),
        Lifecycle::RequestScoped,
    );
    bp.constructor(f!(crate::input::json), Lifecycle::RequestScoped);
    bp
}
