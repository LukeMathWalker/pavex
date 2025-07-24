use pavex::{blueprint::from, Blueprint};

pub fn handler() -> pavex::Response {
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
