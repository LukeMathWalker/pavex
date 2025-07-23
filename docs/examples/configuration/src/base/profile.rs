//! px:profile
pub use pavex::config::ConfigProfile;

#[derive(ConfigProfile)] // px::ann:1
pub enum Profile {
    #[pavex(profile = "dev")]
    Development,
    #[pavex(profile = "prod")]
    Production,
}
