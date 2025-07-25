//! px:register_one
use pavex::Blueprint;

use crate::multiple_named_parameters::FORMAL_GREET;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(FORMAL_GREET); // px::ann:1
    bp // px::skip
}
