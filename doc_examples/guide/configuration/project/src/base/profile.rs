pub use pavex::config::ConfigProfile;

#[derive(ConfigProfile)] // (1)!
pub enum Profile {
    #[pavex(profile = "dev")]
    Development,
    #[pavex(profile = "prod")]
    Production,
}
