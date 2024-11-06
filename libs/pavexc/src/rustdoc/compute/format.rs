use std::io::Read;

#[derive(serde::Deserialize)]
/// A minimal subset of the generated JSON docs for a crate, used to check the format version.
struct CrateMeta {
    format_version: u32,
}

/// Check that the JSON docs we are working with using the expected format version.
pub(super) fn check_format<R: Read>(raw_json: R) -> Result<(), anyhow::Error> {
    let Ok(min_krate) = serde_json::from_reader::<R, CrateMeta>(raw_json) else {
        anyhow::bail!("Failed to deserialize the `format_version` of the generated JSON docs. Is it actually the JSON documentation for a crate?");
    };
    if min_krate.format_version != rustdoc_types::FORMAT_VERSION {
        anyhow::bail!(
            "The JSON docs use the `{}` format version, but `pavexc` expected `{}`.",
            min_krate.format_version,
            rustdoc_types::FORMAT_VERSION,
        );
    }
    Ok(())
}
