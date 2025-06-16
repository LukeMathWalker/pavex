use pavex::blueprint::{from, Blueprint};

pub struct Tied<T, V>(T, V);

#[pavex::request_scoped]
pub fn tied<T>() -> Tied<T, T> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(
    // This can't be built because `tied` can only give you Tied<u8, u8> or Tied<char, char>!
    _tied: Tied<u8, char>,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
