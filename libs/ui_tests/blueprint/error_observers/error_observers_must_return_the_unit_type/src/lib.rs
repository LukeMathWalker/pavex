use pavex::blueprint::Blueprint;

#[pavex::error_observer]
pub fn error_observer() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(ERROR_OBSERVER);
    bp
}
