use anyhow::Context;
use semver::Version;

/// Query GitHub's API to get the latest released version of Pavex.
pub fn latest_released_version() -> Result<Version, anyhow::Error> {
    #[derive(serde::Deserialize)]
    struct Response {
        tag_name: String,
    }

    let response = ureq::get("https://api.github.com/repos/LukeMathWalker/pavex/releases/latest")
        .call()
        .context("Failed to query GitHub's API for the latest release")?;
    if response.status() < 200 || response.status() >= 300 {
        anyhow::bail!(
            "Failed to query GitHub's API for the latest release. It returned an error status code ({})",
            response.status()
        );
    }
    let response: Response = response.into_json()?;
    let version = Version::parse(&response.tag_name)
        .context("Failed to parse the version returned by GitHub's API for the latest release")?;
    Ok(version)
}
