// px:static:start
use pavex::Blueprint;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("pavex.dev").nest(pavex_bp()); // px::hl
    bp // px::skip
}
// px:static:end

fn pavex_bp() -> Blueprint {
    Blueprint::new()
}
