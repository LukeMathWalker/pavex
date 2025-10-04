use pavex::{blueprint::from, Blueprint};

pub struct Unused;

#[pavex::methods]
impl Unused {
    #[request_scoped]
    pub fn new() -> Self {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp
}
