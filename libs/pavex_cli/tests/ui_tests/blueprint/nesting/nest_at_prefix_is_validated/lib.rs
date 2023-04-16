use std::path::PathBuf;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Empty prefix
    bp.nest_at("", sub_blueprint());
    // Prefix does not start with a `/`
    bp.nest_at("api", sub_blueprint());
    bp
}

fn sub_blueprint() -> Blueprint {
    // We don't actually need to register anything here to trigger this diagnostic.
    Blueprint::new()
}
