use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;

pub struct Generic<V>(V);

pub fn once<T>(_generic_input: Generic<T>) -> u8 {
    todo!()
}

pub fn twice<T, S>(_i1: Generic<T>, _i2: Generic<S>) -> u16 {
    todo!()
}

pub fn thrice<T, S, U>(_i1: Generic<T>, _i2: Generic<S>, _i3: Generic<U>) -> u32 {
    todo!()
}

pub mod annotated {
    use super::*;

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
}

pub fn handler(_i: u8, _j: u16, _k: u32, _l: u64, _m: u128, _n: bool) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate::annotated]);
    bp.request_scoped(f!(crate::once));
    bp.request_scoped(f!(crate::twice));
    bp.request_scoped(f!(crate::thrice));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
