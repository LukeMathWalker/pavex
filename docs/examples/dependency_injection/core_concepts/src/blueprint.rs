use pavex::Blueprint;
use pavex::blueprint::from;
use pavex::methods;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}

pub struct A;

#[methods]
impl A {
    #[request_scoped]
    pub fn new() -> Self {
        A
    }
}

pub struct B;

#[methods]
impl B {
    #[request_scoped]
    pub fn new() -> Self {
        B
    }
}
