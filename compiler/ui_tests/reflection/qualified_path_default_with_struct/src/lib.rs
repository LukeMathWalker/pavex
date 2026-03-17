use dep::{Keyed, User};
use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn get_keyed() -> Keyed<User> {
    Keyed(std::marker::PhantomData)
}

#[pavex::get(path = "/")]
pub fn handler(_keyed: Keyed<User>) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
