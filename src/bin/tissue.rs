use clap::{Args, Parser, Subcommand};
use std::{fs, path::PathBuf, process::exit};
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
	Add(Add),
	/// Append to an existing tissue's description by index
	Describe(Describe),
	/// Add a tag to an existing tissue by index
	Tag(Describe),
	/// Delete an existing tissue by index
	Remove(Remove),
	Commit(Index),
	Publish(Index),
}

#[derive(Args)]
struct Index {
	index: usize,
}

#[derive(Args)]
struct Add {
	/// Title of the new issue.
	///
	/// This should be formatted as a prospective git commit or issue title.
	title: String,
}

#[derive(Args)]
struct Describe {
	/// Description of tissue
	with: String,
	/// Index of tissue to describe
	index: usize,
}

#[derive(Args)]
struct Remove {
	/// Which tissue to delete
	index: usize,
	/// Remove a single field, instead of the whole tissue
	#[command(subcommand)]
	which: Option<WhichRemove>,
}

#[derive(Subcommand)]
enum WhichRemove {
	/// Remove a description
	Description(Index),
	/// Remove a tag
	Tag(RemoveTag),
}

#[derive(Args)]
struct RemoveTag {
	tag: String,
}

fn try_get(tissue_box: &tissuebox::Box, index: usize) -> &tissuebox::Tissue {
	let Some(tissue) = tissue_box.get(index) else {
		error!("no tissue with index {index}");
		exit(1);
	};
	tissue
}

fn try_get_mut(tissue_box: &mut tissuebox::Box, index: usize) -> &mut tissuebox::Tissue {
	let Some(tissue) = tissue_box.get_mut(index) else {
		error!("no tissue with index {index}");
		exit(1);
	};
	tissue
}

fn main() {
	tracing_subscriber::fmt::init();
	let cli = Cli::parse();

	// Load tissue box
	let tissue_box_toml = fs::read_to_string(&cli.input).unwrap_or_else(|msg| {
		error!("failed to read {}: {msg}", cli.input.display());
		exit(1);
	});
	let mut tissue_box: tissuebox::Box = toml::from_str(&tissue_box_toml).unwrap_or_else(|msg| {
		error!(
			"failed to parse {} as tissue box: {msg}",
			cli.input.display()
		);
		exit(1);
	});

	// Update tissue box
	match cli.command {
		Some(Command::List) => print!("{tissue_box}"),
		Some(Command::Add(Add { title })) => tissue_box.create(title),
		Some(Command::Describe(Describe { index, with })) => {
			try_get_mut(&mut tissue_box, index).describe(with);
		}
		Some(Command::Tag(Describe { index, with })) => {
			try_get_mut(&mut tissue_box, index).tag(with);
		}
		Some(Command::Remove(Remove { index, which: None })) => {
			if tissue_box.remove(index).is_none() {
				error!("no tissue with index {index}");
				exit(1);
			};
		}
		Some(Command::Remove(Remove {
			index: tissue_index,
			which: Some(WhichRemove::Description(Index { index })),
		})) => {
			let tissue = try_get_mut(&mut tissue_box, tissue_index);
			if tissue.description.get(index).is_none() {
				error!("no description with index {index} on tissue {index}");
				exit(1);
			}
			tissue.description.remove(index);
		}
		Some(Command::Remove(Remove {
			index,
			which: Some(WhichRemove::Tag(RemoveTag { tag })),
		})) => {
			if !try_get_mut(&mut tissue_box, index).tags.remove(&tag) {
				error!("no tag named {tag}");
				exit(1);
			}
		}
		Some(Command::Commit(Index { index })) => {
			match try_get_mut(&mut tissue_box, index).commit() {
				Ok(()) => {
					let _ = tissue_box.remove(index);
				}
				Err(msg) => {
					error!("failed to commit: {msg}");
					exit(1);
				}
			}
		}
		Some(Command::Publish(Index { index })) => {
			match try_get_mut(&mut tissue_box, index).publish() {
				Ok(()) => {
					let _ = tissue_box.remove(index);
				}
				Err(msg) => {
					error!("failed to publish: {msg}");
					exit(1);
				}
			}
		}
		None => {
			{
				use std::panic;

				let original_hook = panic::take_hook();
				panic::set_hook(Box::new(move |panic_info| {
					// intentionally ignore errors here since we're already in a panic
					let _ = crossterm::terminal::disable_raw_mode();
					let _ = crossterm::execute!(
						std::io::stdout(),
						crossterm::terminal::LeaveAlternateScreen
					);
					original_hook(panic_info);
				}));
			}
			if let Err(msg) = tissuebox::tui::run(&mut tissue_box) {
				error!("{msg}");
				exit(1);
			}
		}
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
