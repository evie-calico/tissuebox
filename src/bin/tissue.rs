use clap::{Args, Parser, Subcommand};
use std::{collections::HashSet, fs, path::PathBuf, process::exit};
use tracing::error;

#[derive(Parser)]
struct Cli {
    #[clap(short, long, default_value = ".tissuebox")]
    input: PathBuf,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Display formatted tissuebox
    List,
    /// Create new tissue
    Add(AddArgs),
    /// Append to an existing tissue's description by index
    Describe(DescribeArgs),
    /// Add a tag to an existing tissue by index
    Tag(DescribeArgs),
    /// Delete an existing tissue by index
    Remove(RemoveArg),
}

#[derive(Args)]
struct RemoveArg {
    // index of tissue to delete
    index: usize,
}

#[derive(Args)]
struct AddArgs {
    // title of tissue to create
    title: String,
}

#[derive(Args)]
struct DescribeArgs {
    // index of tissue to describe
    index: usize,
    // description of tissue
    with: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tissue {
    pub title: String,
    #[serde(default)]
    pub description: Vec<String>,
    #[serde(default)]
    pub tags: HashSet<String>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TissueBox {
    #[serde(default)]
    tissues: Vec<Tissue>,
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Load tissue box
    let tissue_box_toml = fs::read_to_string(&cli.input).unwrap_or_else(|msg| {
        error!("failed to read {}: {msg}", cli.input.display());
        exit(1);
    });
    let mut tissue_box: TissueBox = toml::from_str(&tissue_box_toml).unwrap_or_else(|msg| {
        error!(
            "failed to parse {} as tissue box: {msg}",
            cli.input.display()
        );
        exit(1);
    });

    // Update tissue box
    match cli.command {
        Some(Command::List) => {
            for (
                index,
                Tissue {
                    title,
                    description,
                    tags,
                },
            ) in tissue_box.tissues.iter().enumerate()
            {
                print!("{index}. {title}");
                if !tags.is_empty() {
                    print!(
                        " ({})",
                        tags.iter().cloned().collect::<Vec<String>>().join(", ")
                    );
                }
                println!();
                for description in description {
                    println!("\t- {description}");
                }
            }
        }
        Some(Command::Add(AddArgs { title })) => {
            tissue_box.tissues.push(Tissue {
                title,
                ..Default::default()
            });
        }
        Some(Command::Describe(DescribeArgs { index, with })) => {
            let Some(tissue) = tissue_box.tissues.get_mut(index) else {
                error!("no tissue with index {index}");
                exit(1);
            };
            tissue.description.push(with);
        }
        Some(Command::Tag(DescribeArgs { index, with })) => {
            let Some(tissue) = tissue_box.tissues.get_mut(index) else {
                error!("no tissue with index {index}");
                exit(1);
            };
            tissue.tags.insert(with);
        }
        Some(Command::Remove(RemoveArg { index })) => {
            let Some(_tissue) = tissue_box.tissues.get(index) else {
                error!("no tissue with index {index}");
                exit(1);
            };
            tissue_box.tissues.remove(index);
        }
        None => todo!(),
    }

    // Save tissue box
    let tissue_box_toml = toml::to_string(&tissue_box).unwrap_or_else(|msg| {
        error!("failed to serialize tissue box: {msg}");
        exit(1);
    });
    fs::write(&cli.input, tissue_box_toml).unwrap_or_else(|msg| {
        error!("failed to write tissue box: {msg}");
    });
}
