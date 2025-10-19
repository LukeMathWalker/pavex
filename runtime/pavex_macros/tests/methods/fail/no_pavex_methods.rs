use pavex_macros::methods;

pub struct A;

#[methods]
impl A {
    pub fn new() -> Self {
        A
    }
    
    pub fn regular_method(&self) -> i32 {
        69
    }
}

fn main() {}