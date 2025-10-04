use pavex_macros::from;

mod a {}

fn main() {
    let _ = from![crate:a];
}
