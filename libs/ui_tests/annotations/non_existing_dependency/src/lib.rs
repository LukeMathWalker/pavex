use pavex::{blueprint::from, Blueprint};

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![
        // Non-existing dependency.
        non_existing_dep,
    ]);
    bp
}
