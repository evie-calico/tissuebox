use clap::{Args, Parser, Subcommand};
use std::{path::PathBuf, process::exit};
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
	List(List),
	/// Create new tissue
	Add(Add),
	/// Append to an existing tissue's description by index
	Describe(Describe),
	/// Add a tag to an existing tissue by index
	Tag(Tag),
	/// Delete an existing tissue by index
	Remove(Remove),
	/// Commit a tissue to git by index
	Commit(Index),
	/// Publish a tissue to GitHub by index
	Publish(Index),
}

#[derive(Args)]
struct Index {
	index: usize,
}

#[derive(Args)]
struct OptionIndex {
	index: Option<usize>,
}

#[derive(Args)]
struct List {
	index: Option<usize>,
	#[command(subcommand)]
	which: Option<WhichList>,
}

#[derive(Subcommand)]
enum WhichList {
	/// List title
	Title,
	/// List descriptions
	Description(OptionIndex),
	/// List all tags
	Tags,
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
	description: String,
	/// Index of tissue to describe
	index: usize,
}

#[derive(Args)]
struct Tag {
	tag: String,
	/// Index of tissue to tag
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
	Tag(TagName),
}

#[derive(Args)]
struct TagName {
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
	let mut tissue_box = tissuebox::Box::open(&cli.input).unwrap_or_else(|msg| {
		error!("failed to open {}: {msg}", cli.input.display());
		exit(1);
	});

	let save = |tissue_box: &tissuebox::Box| {
		tissue_box.save(&cli.input).unwrap_or_else(|msg| {
			error!("failed to serialize tissue box: {msg}");
			exit(1);
		});
	};

	// Update tissue box
	match cli.command {
		Some(Command::List(List { index: None, which: None })) => print!("{tissue_box}"),
		Some(Command::List(List { index: Some(index), which: None })) => print!("{}", try_get(&tissue_box, index)),
		Some(Command::List(List {
			index: Some(index),
			which: Some(WhichList::Title),
		})) => {
			println!("{}", try_get(&tissue_box, index).title);
		}
		Some(Command::List(List {
			index: Some(index),
			which: Some(WhichList::Description(OptionIndex { index: None })),
		})) => {
			println!("{}", try_get(&tissue_box, index).description.join("\n"));
		}
		Some(Command::List(List {
			index: Some(tissue_index),
			which: Some(WhichList::Description(OptionIndex { index: Some(index) })),
		})) => {
			println!(
				"{}",
				try_get(&tissue_box, tissue_index).description.get(index).unwrap_or_else(|| {
					error!("no description with index {index} on tissue {index}");
					exit(1);
				})
			)
		}
		Some(Command::List(List {
			index: Some(index),
			which: Some(WhichList::Tags),
		})) => {
			let tissue = try_get(&tissue_box, index);
			let mut iter = tissue.tags.iter();
			if let Some(first) = iter.next() {
				print!("{first}");
				for next in iter {
					print!(", {next}");
				}
				println!();
			}
		}
		Some(Command::List(List { index: None, which: Some(_) })) => panic!("list subcommand specified without index"),
		Some(Command::Add(Add { title })) => {
			tissue_box.create(title);
			save(&tissue_box);
		}
		Some(Command::Describe(Describe { index, description })) => {
			try_get_mut(&mut tissue_box, index).describe(description);
			save(&tissue_box);
		}
		Some(Command::Tag(Tag { index, tag })) => {
			try_get_mut(&mut tissue_box, index).tag(tag);
			save(&tissue_box);
		}
		Some(Command::Remove(Remove { index, which: None })) => {
			if tissue_box.remove(index).is_none() {
				error!("no tissue with index {index}");
				exit(1);
			};
			save(&tissue_box);
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
			save(&tissue_box);
		}
		Some(Command::Remove(Remove {
			index,
			which: Some(WhichRemove::Tag(TagName { tag })),
		})) => {
			if !try_get_mut(&mut tissue_box, index).tags.remove(&tag) {
				error!("no tag named {tag}");
				exit(1);
			}
			save(&tissue_box);
		}
		Some(Command::Commit(Index { index })) => {
			try_get_mut(&mut tissue_box, index).commit().unwrap_or_else(|msg| {
				error!("failed to commit: {msg}");
				exit(1);
			});
			save(&tissue_box);
		}
		Some(Command::Publish(Index { index })) => {
			try_get_mut(&mut tissue_box, index).publish().unwrap_or_else(|msg| {
				error!("failed to publish: {msg}");
				exit(1);
			});
			save(&tissue_box);
		}
		None => {
			{
				use std::panic;

				let original_hook = panic::take_hook();
				panic::set_hook(Box::new(move |panic_info| {
					// intentionally ignore errors here since we're already in a panic
					let _ = crossterm::terminal::disable_raw_mode();
					let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
					original_hook(panic_info);
				}));
			}
			if let Err(msg) = tissuebox::tui::run(&mut tissue_box, &cli.input) {
				error!("{msg}");
				exit(1);
			}
		}
	}
}
