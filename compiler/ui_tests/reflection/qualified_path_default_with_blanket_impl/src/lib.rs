use dep::Container;
use pavex::{blueprint::from, Blueprint};

pub struct MyType;

#[pavex::request_scoped]
pub fn get_container() -> Container<MyType> {
    Container(std::marker::PhantomData)
}

#[pavex::get(path = "/")]
pub fn handler(_c: Container<MyType>) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
