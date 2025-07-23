// px:intro:start
use pavex::Blueprint;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    // Serve the website from the root domain...
    bp.domain("pavex.dev").nest(website_bp()); // px::hl
    // ...while reserving a subdomain for the REST API.
    bp.domain("api.pavex.dev").prefix("/v1").nest(api_bp()); // px::hl
    bp // px::skip
}
// px:intro:end

fn website_bp() -> Blueprint {
    Blueprint::new()
}

fn api_bp() -> Blueprint {
    Blueprint::new()
}
