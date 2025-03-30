use pavex::blueprint::{from, Blueprint};

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![non_existing_dep]);
    bp
}
