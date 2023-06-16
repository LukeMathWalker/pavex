use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // The prefix cannot be empty
    bp.nest_at("", sub_blueprint());
    // If the prefix is not empty, it **must** start with a `/`
    bp.nest_at("api", sub_blueprint());
    // If the prefix is not empty, it **cannot** end with a `/`
    bp.nest_at("/api/", sub_blueprint());
    bp
}

fn sub_blueprint() -> Blueprint {
    // We don't actually need to register anything here to trigger this diagnostic.
    Blueprint::new()
}
