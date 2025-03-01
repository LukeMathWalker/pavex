use pavex::config::ConfigProfile;

/// The configuration profile, i.e. a way to determine which set of
/// configuration values should be used.
///
/// Check out [the guide](https://pavex.dev/docs/guide/configuration/loading/#configuration-profile)
/// for more details on configuration profiles.
#[derive(ConfigProfile, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    Dev,
    Prod,
}
