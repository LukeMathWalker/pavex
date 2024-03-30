pub use blueprint::blueprint;
pub use handler::handler;

mod blueprint;
mod handler;

pub struct A;
pub struct B;

pub fn a() -> A {
    A
}
pub fn b() -> B {
    B
}
