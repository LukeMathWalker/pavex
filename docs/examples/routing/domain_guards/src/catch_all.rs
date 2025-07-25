// px:catch_all:start
use pavex::Blueprint;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{*any}.example.dev").nest(sub_bp()); // px::hl
    bp // px::skip
}
// px:catch_all:end

fn sub_bp() -> Blueprint {
    Blueprint::new()
}
