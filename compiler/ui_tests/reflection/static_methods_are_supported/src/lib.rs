use pavex::{blueprint::from, Blueprint};

pub struct Streamer;

#[pavex::methods]
impl Streamer {
    #[get(path = "/")]
    pub fn stream_file() -> pavex::Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
