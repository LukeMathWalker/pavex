use std::sync::{Arc, Mutex, RwLock};

use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::http::StatusCode;

pub struct Custom;

pub fn arc() -> Arc<Custom> {
    Arc::new(Custom)
}

pub fn arc_mutex() -> Arc<Mutex<Custom>> {
    Arc::new(Mutex::new(Custom))
}

pub fn arc_rwlock() -> Arc<RwLock<Custom>> {
    Arc::new(RwLock::new(Custom))
}

pub fn handler(_s: &Arc<Custom>, _t: &Arc<Mutex<Custom>>, _u: &Arc<RwLock<Custom>>) -> StatusCode {
    todo!()
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
    bp.singleton(f!(crate::arc));
    bp.singleton(f!(crate::arc_mutex));
    bp.singleton(f!(crate::arc_rwlock));
    bp.routes(from![crate]);
    bp
}
