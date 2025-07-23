//! px:register_one
use crate::pool::DB_CONNECTION_POOL;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(DB_CONNECTION_POOL); // px::ann:1
    bp // px::skip
}
