use pavex_macros::from;

pub struct A<T>(T);

fn main() {
    let _ = from![crate::A::<String>];
}
