use clap::Parser;
use std::panic;
use std::process::exit;
use tissuebox::prelude::*;
use tracing::error;

fn main() {
	tracing_subscriber::fmt::init();
	let cli = Cli::parse();
	let mut tissue_box = TissueBox::open(&cli.input).unwrap_or_else(|msg| {
		error!("failed to open {}: {msg}", cli.input.display());
		exit(1);
	});

	// Update tissue box
	match cli.command {
		Some(command) => {
			match cli::run(command, &mut tissue_box) {
				Ok(Some(out)) => print!("{out}"),
				Ok(None) => {}
				Err(msg) => {
					error!("{msg}");
					exit(1);
				}
			}
			// cli::run can't manage saving because it needs to be run in unit tests,
			// so just save after every run.
			if let Err(msg) = tissue_box.save(&cli.input) {
				error!("failed to serialize tissue box: {msg}");
				exit(1);
			};
		}
		None => {
			let original_hook = panic::take_hook();
			panic::set_hook(Box::new(move |panic_info| {
				// intentionally ignore errors here since we're already in a panic
				let _ = crossterm::terminal::disable_raw_mode();
				let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
				original_hook(panic_info);
			}));
			if let Err(msg) = tissuebox::tui::run(&mut tissue_box, &cli.input) {
				error!("{msg}");
				exit(1);
			}
		}
	}
}
