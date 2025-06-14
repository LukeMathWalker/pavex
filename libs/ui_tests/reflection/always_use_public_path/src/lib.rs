use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

pub use private::*;

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    register(&mut bp);
    bp.routes(from![crate]);
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

    impl Default for B {
        fn default() -> Self {
            Self::new()
        }
    }

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
