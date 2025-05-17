use pavex_macros::get;

#[get(path = "/a")]
fn a() {
    todo!()
}

#[get(path = "/b")]
pub(crate) fn b() {
    todo!()
}

fn main() {}
