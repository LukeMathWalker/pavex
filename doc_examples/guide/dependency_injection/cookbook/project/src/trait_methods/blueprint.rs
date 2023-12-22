use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(<crate::User as crate::trait_methods::WithId>::id),
        Lifecycle::RequestScoped,
    );
    bp.constructor(f!(crate::functions::extract), Lifecycle::RequestScoped);
    bp
}
