use pavex::{blueprint::from, Blueprint};

pub fn handler() -> String {
    todo!()
}

#[pavex::get(path = "/")]
pub fn nested_handler() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    // Invalid domain!
    bp.domain("s{.com").nest({
        let mut bp = Blueprint::new();
        bp.routes(from![crate]);
        bp
    });
    bp
}
