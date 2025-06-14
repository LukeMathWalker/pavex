use pavex::blueprint::Blueprint;
use pavex::f;

pub fn my_f() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f![my_f]);
    bp
}
