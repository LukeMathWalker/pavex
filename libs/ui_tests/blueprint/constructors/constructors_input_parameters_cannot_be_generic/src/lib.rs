use pavex::{blueprint::from, Blueprint};

pub struct Generic<V>(V);

#[pavex::request_scoped]
pub fn once<T>(_generic_input: Generic<T>) -> u64 {
    todo!()
}

#[pavex::transient]
pub fn twice<T, S>(_i1: Generic<T>, _i2: Generic<S>) -> u128 {
    todo!()
}

#[pavex::singleton]
pub fn thrice<T, S, U>(_i1: Generic<T>, _i2: Generic<S>, _i3: Generic<U>) -> bool {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_l: u64, _m: u128, _n: bool) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
