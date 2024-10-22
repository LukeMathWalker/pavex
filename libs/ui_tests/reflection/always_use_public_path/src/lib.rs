use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub use private::*;

pub fn handler(_a: A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register(&mut bp);
    bp.route(GET, "/", f!(self::handler));
    bp
}

mod private {
    use pavex::blueprint::Blueprint;
    use pavex::f;

    pub struct A;

    pub fn register(bp: &mut Blueprint) {
        bp.request_scoped(f!(self::a));
    }

    pub fn a() -> A {
        todo!()
    }
}
