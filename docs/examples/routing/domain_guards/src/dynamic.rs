// px:dynamic:start
use pavex::Blueprint;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{sub}.pavex.dev").nest(sub_bp()); // px::hl
    bp // px::skip
}
// px:dynamic:end

fn sub_bp() -> Blueprint {
    Blueprint::new()
}
