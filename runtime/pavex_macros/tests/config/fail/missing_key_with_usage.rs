use pavex_macros::config;

#[config]
pub struct MyConfig {
    pub value: String,
}

fn main() {
    let _config = MyConfig {
        value: "test".to_string(),
    };
}
