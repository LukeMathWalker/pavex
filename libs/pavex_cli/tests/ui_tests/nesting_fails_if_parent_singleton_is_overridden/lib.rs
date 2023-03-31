use std::path::PathBuf;

use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::singleton), Lifecycle::Singleton);
    bp.route(GET, "/parent", f!(crate::handler));
    bp.nest(sub_blueprint());
    bp
}

pub fn singleton() -> u64 {
    todo!()
}

pub fn overridden_singleton() -> u64 {
    todo!()
}

pub fn handler(_x: u64) -> String {
    todo!()
}

fn sub_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::overridden_singleton), Lifecycle::Singleton);
    bp.route(GET, "/child", f!(crate::handler));
    bp
}
