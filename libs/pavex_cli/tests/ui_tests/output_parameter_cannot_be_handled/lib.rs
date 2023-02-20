use pavex_builder::{f, Blueprint};

pub fn c() -> Box<dyn std::error::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(f!(crate::c), "/home");
    bp
}
