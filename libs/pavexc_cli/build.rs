use std::error::Error;

use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        // Emit VERGEN_GIT_SHA
        .git_sha(true)
        .emit()?;
    Ok(())
}
