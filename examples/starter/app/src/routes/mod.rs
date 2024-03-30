pub mod ping;

use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn register(bp: &mut Blueprint) {
    bp.route(GET, "/api/ping", f!(self::ping::get));
}
