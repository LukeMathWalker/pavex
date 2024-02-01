use pavex::blueprint::Blueprint;
use pavex::f;

pub fn error_observer() {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(f!(crate::error_observer));
    bp
}
