pub struct A;

#[pavex::methods]
impl A {
    #[pavex::get(path = "/test")]
    pub fn handler(&self) -> String {
        "Hello".to_string()
    }
    
    pub fn regular_method(&self) -> i32 {
        42
    }
}

fn main() {}