use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(crate::non_static_methods::UserStore::retrieve),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(crate::non_static_methods::UserStore::new),
        Lifecycle::Singleton,
    );
    bp
}
