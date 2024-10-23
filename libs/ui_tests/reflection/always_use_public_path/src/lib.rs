use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub use private::*;

pub fn handler(_a: A, _b: B) -> Response {
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

    pub fn a() -> A {
        todo!()
    }

    pub struct B;

    impl B {
        pub fn new() -> B {
            todo!()
        }
    }

    pub fn register(bp: &mut Blueprint) {
        bp.request_scoped(f!(self::a));
        bp.request_scoped(f!(self::B::new));
    }
}
