pub struct A;

#[pavex::methods]
impl A {
    pub fn new() -> Self {
        A
    }
    
    pub fn regular_method(&self) -> i32 {
        42
    }
}

fn main() {}