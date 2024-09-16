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
	// description of tissue
	with: String,
	// index of tissue to describe
	index: usize,
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
		Some(Command::Add(AddArgs { title })) => tissue_box.create(title),
		Some(Command::Describe(DescribeArgs { index, with })) => {
			let Some(tissue) = tissue_box.get_mut(index) else {
				error!("no tissue with index {index}");
				exit(1);
			};
			tissue.description.push(with);
		}
		Some(Command::Tag(DescribeArgs { index, with })) => {
			let Some(tissue) = tissue_box.get_mut(index) else {
				error!("no tissue with index {index}");
				exit(1);
			};
			tissue.tags.insert(with);
		}
		Some(Command::Remove(RemoveArg { index })) => {
			if tissue_box.remove(index).is_none() {
				error!("no tissue with index {index}");
				exit(1);
			};
		}
		None => {
			{
				use crossterm::execute;
				use crossterm::terminal::disable_raw_mode;
				use std::panic;

				let original_hook = panic::take_hook();
				panic::set_hook(Box::new(move |panic_info| {
					// intentionally ignore errors here since we're already in a panic

					let _ = disable_raw_mode();
					let _ = execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
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
