use pavex_builder::{f, router::GET, Blueprint, Lifecycle};

pub struct Tied<T, V>(T, V);

pub fn tied<T>() -> Tied<T, T> {
    todo!()
}

pub fn handler(
    // This can't be built because `tied` can only give you Tied<u8, u8> or Tied<char, char>!
    tied: Tied<u8, char>,
) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::tied), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
