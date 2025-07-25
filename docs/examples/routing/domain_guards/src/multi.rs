// px:multi:start
use pavex::Blueprint;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{user_id}.{tenant_id}.pavex.dev").nest(user_bp()); // px::hl
    bp // px::skip
}
// px:multi:end

fn user_bp() -> Blueprint {
    Blueprint::new()
}
