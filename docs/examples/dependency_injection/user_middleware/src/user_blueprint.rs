//! px:register_one
use crate::user::USER_EXTRACT;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(USER_EXTRACT); // px::ann:1
    bp // px::skip
}
