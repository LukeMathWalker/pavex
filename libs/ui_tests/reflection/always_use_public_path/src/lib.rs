use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

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
    use pavex::{blueprint::from, Blueprint};

    pub struct A;

    #[pavex::request_scoped(id = "A_")]
    pub fn a() -> A {
        todo!()
    }

    pub struct B;

    impl Default for B {
        fn default() -> Self {
            Self::new()
        }
    }

    #[pavex::methods]
    impl B {
        #[request_scoped]
        pub fn new() -> B {
            todo!()
        }
    }

    pub fn register(bp: &mut Blueprint) {
        bp.import(from![crate::private]);
    }
}
