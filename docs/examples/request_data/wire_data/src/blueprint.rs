use pavex::{Blueprint, blueprint::from};

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex]);
    bp.routes(from![crate]);
    bp
}
