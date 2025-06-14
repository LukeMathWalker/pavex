use pavex::blueprint::{constructor::Lifecycle, from, Blueprint};
use pavex::f;

pub struct Tied<T, V>(T, V);

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
    bp.constructor(f!(crate::tied), Lifecycle::RequestScoped);
    bp.routes(from![crate]);
    bp
}
