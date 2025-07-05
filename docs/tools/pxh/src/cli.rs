use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pxh", version, about = "Pavex contributor CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage examples shown in the documentation at pavex.dev
    Example(ExampleCommand),
    /// Manage tutorials bundled in the documentation at pavex.dev
    Tutorial(TutorialCommand),
}

#[derive(Parser)]
pub struct ExampleCommand {
    #[command(subcommand)]
    pub command: ExampleSubcommand,
}

#[derive(Subcommand)]
pub enum ExampleSubcommand {
    /// Extract all snippets from the example code, including compilation output.
    ///
    /// Existing snippets are overwritten, if they are out of date.
    Regenerate,
    /// Verify that all snippets extracted from the example code are up to date.
    Verify,
}

#[derive(Parser)]
pub struct TutorialCommand {
    #[command(subcommand)]
    pub command: TutorialSubcommand,
}

#[derive(Subcommand)]
pub enum TutorialSubcommand {
    Hydrate,
    Extract,
}
