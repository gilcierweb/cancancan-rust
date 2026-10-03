use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cancancan", about = "Scaffolding for cancancan-rust abilities")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Writes a starter ability module to the output path.
    Scaffold {
        /// Destination file for the generated ability module.
        #[arg(long, default_value = "src/ability.rs")]
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Scaffold { output } => {
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(error) = std::fs::create_dir_all(parent) {
                        eprintln!("cannot create output directory: {error}");
                        std::process::exit(1);
                    }
                }
            }
            if let Err(error) = std::fs::write(&output, render_template()) {
                eprintln!("cannot write ability template: {error}");
                std::process::exit(1);
            }
            println!("ability scaffold written to {}", output.display());
        }
    }
}

fn render_template() -> String {
    include_str!("template.rs").to_owned()
}

#[cfg(test)]
mod tests {
    use super::render_template;

    #[test]
    fn template_defines_an_ability_builder() {
        let template = render_template();
        assert!(template.contains("pub fn build_ability"));
        assert!(template.contains("Ability::new"));
    }
}
