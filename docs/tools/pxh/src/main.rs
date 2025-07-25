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
            ExampleSubcommand::Regenerate { skip_compilation } => {
                process_examples(&cwd, false, skip_compilation)
            }
            ExampleSubcommand::Verify => process_examples(&cwd, true, false),
        },
        Command::Tutorial(tutorial) => match tutorial.command {
            TutorialSubcommand::Hydrate { skip_compilation } => {
                hydrate_tutorials(&cwd, skip_compilation)
            }
            TutorialSubcommand::Extract => extract_patches(&cwd),
        },
    }
}
