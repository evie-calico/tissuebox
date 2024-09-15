use clap::{Args, Parser, Subcommand};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::exit,
};
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
    pub tags: HashSet<String>,
    pub description: String,
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Load tissue box
    let tissue_box_toml = fs::read_to_string(&cli.input).unwrap_or_else(|msg| {
        error!("failed to read {}: {msg}", cli.input.display());
        exit(1);
    });
    let mut tissue_box: HashMap<String, Tissue> =
        toml::from_str(&tissue_box_toml).unwrap_or_else(|msg| {
            error!(
                "failed to parse {} as tissue box: {msg}",
                cli.input.display()
            );
            exit(1);
        });

    // Update tissue box
    match cli.command {
        Some(Command::List) => {
            let mut tissues: Vec<_> = tissue_box.iter_mut().collect();
            tissues.sort_unstable_by(|a, b| a.0.cmp(b.0));
            for (index, (tissue, Tissue { tags, description })) in tissues.into_iter().enumerate() {
                let tags = tags.iter().cloned().collect::<Vec<String>>().join(", ");
                print!("{index}. {tissue}");
                if !tags.is_empty() {
                    print!(" ({tags})");
                }
                println!();
                if !description.is_empty() {
                    println!("\t- {description}");
                }
            }
        }
        Some(Command::Add(AddArgs { title })) => {
            tissue_box.insert(title, Tissue::default());
        }
        Some(Command::Describe(DescribeArgs { index, with })) => {
            let mut tissues: Vec<_> = tissue_box.iter_mut().collect();
            tissues.sort_unstable_by(|a, b| a.0.cmp(b.0));
            let (_, tissue) = tissues.get_mut(index).unwrap_or_else(|| {
                error!("no tissue with index {index}");
                exit(1);
            });
            tissue.description += &with;
        }
        Some(Command::Tag(DescribeArgs { index, with })) => {
            let mut tissues: Vec<_> = tissue_box.iter_mut().collect();
            tissues.sort_unstable_by(|a, b| a.0.cmp(b.0));
            let (_, tissue) = tissues.get_mut(index).unwrap_or_else(|| {
                error!("no tissue with index {index}");
                exit(1);
            });
            tissue.tags.insert(with);
        }
        Some(Command::Remove(RemoveArg { index })) => {
            let mut tissues: Vec<_> = tissue_box.iter_mut().collect();
            tissues.sort_unstable_by(|a, b| a.0.cmp(b.0));
            let tissue = tissues
                .get_mut(index)
                .unwrap_or_else(|| {
                    error!("no tissue with index {index}");
                    exit(1);
                })
                .0
                .clone();
            tissue_box.remove(tissue.as_str());
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
