use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        // Emit VERGEN_GIT_DESCRIBE
        .git_describe(true, false, None)
        .emit()?;
    Ok(())
}
