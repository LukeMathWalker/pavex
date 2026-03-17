use pavex::{blueprint::from, Blueprint};

pub struct NumericConst<const N: usize>;
pub struct BoolConst<const B: bool>;
pub struct CharConst<const C: char>;

#[pavex::request_scoped]
pub fn numeric() -> NumericConst<8> {
    NumericConst
}

#[pavex::request_scoped]
pub fn boolean() -> BoolConst<true> {
    BoolConst
}

#[pavex::request_scoped]
pub fn character() -> CharConst<'a'> {
    CharConst
}

#[pavex::get(path = "/")]
pub fn handler(
    _n: NumericConst<8>,
    _b: BoolConst<true>,
    _c: CharConst<'a'>,
) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
