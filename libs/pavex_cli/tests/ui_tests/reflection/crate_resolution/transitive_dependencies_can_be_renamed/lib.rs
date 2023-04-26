use pavex_builder::{constructor::Lifecycle, f, router::GET, Blueprint};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    dep::dep_blueprint(&mut bp);
    bp
}
