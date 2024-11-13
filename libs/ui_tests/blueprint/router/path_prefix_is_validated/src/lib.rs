use pavex::blueprint::Blueprint;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // The prefix cannot be empty
    bp.prefix("").nest(sub_blueprint());
    // If the prefix is not empty, it **must** start with a `/`
    bp.prefix("api").nest(sub_blueprint());
    // If the prefix is not empty, it **cannot** end with a `/`
    bp.prefix("/api/").nest(sub_blueprint());
    bp
}

fn sub_blueprint() -> Blueprint {
    // We don't actually need to register anything here to trigger this diagnostic.
    Blueprint::new()
}
