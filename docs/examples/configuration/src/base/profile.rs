//! px:profile
pub use pavex::config::ConfigProfile;

#[derive(ConfigProfile)] // px::ann:1
pub enum Profile {
    #[px(profile = "dev")]
    Development,
    #[px(profile = "prod")]
    Production,
}
