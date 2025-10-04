use pavex::http::StatusCode;
use pavex::{blueprint::from, Blueprint};
use std::sync::{Arc, Mutex, RwLock};

pub struct Custom;

#[pavex::singleton]
pub fn arc() -> Arc<Custom> {
    Arc::new(Custom)
}

#[pavex::singleton]
pub fn arc_mutex() -> Arc<Mutex<Custom>> {
    Arc::new(Mutex::new(Custom))
}

#[pavex::singleton]
pub fn arc_rwlock() -> Arc<RwLock<Custom>> {
    Arc::new(RwLock::new(Custom))
}

#[pavex::get(path = "/")]
pub fn route_handler(
    _s: &Arc<Custom>,
    _t: &Arc<Mutex<Custom>>,
    _u: &Arc<RwLock<Custom>>,
) -> StatusCode {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
