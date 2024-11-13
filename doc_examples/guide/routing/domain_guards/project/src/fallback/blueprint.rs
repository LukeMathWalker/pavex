use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("pavex.dev").nest(website_bp());
    bp.domain("api.pavex.dev").prefix("/v1").nest(api_bp());
    // If no domain matches, serve a 404 page.
    bp.fallback(f!(super::unknown_domain));
    bp
}

fn website_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(pavex::blueprint::router::GET, "/", f!(super::index));
    // Other web pages...
    bp
}

fn api_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(pavex::blueprint::router::GET, "/users", f!(super::users));
    // Other API routes...
    bp
}
