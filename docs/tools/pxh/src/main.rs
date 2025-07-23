use clap::Parser;
use pxh::{
    cli::{Cli, Command, ExampleSubcommand, TutorialSubcommand},
    examples::process_examples,
    tutorials::{extract_patches, hydrate_tutorials},
};

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().unwrap();

    match cli.command {
        Command::Example(example) => match example.command {
            ExampleSubcommand::Regenerate => process_examples(&cwd, false),
            ExampleSubcommand::Verify => process_examples(&cwd, true),
        },
        Command::Tutorial(tutorial) => match tutorial.command {
            TutorialSubcommand::Hydrate => hydrate_tutorials(&cwd),
            TutorialSubcommand::Extract => extract_patches(&cwd),
        },
    }
}
