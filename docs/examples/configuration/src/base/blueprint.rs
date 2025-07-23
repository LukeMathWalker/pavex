//! px:register_one
use super::DATABASE_CONFIG;
use pavex::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.config(DATABASE_CONFIG); // px::ann:1
    bp // px::skip
}
