use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use clap::Parser;
use std::process::exit;
use std::{env, panic};
use tissuebox::prelude::*;
use tracing::error;

fn main() {
	{
		let mut args = env::args().skip(1);
		if args.next().as_deref() == Some(tissuebox::DAEMONIZE_ARG) {
			let text = args.next().unwrap();
			#[cfg(target_os = "linux")]
			Clipboard::new().unwrap().set().wait().text(text).unwrap();
			#[cfg(not(target_os = "linux"))]
			Clipboard::new().unwrap().set_text(text).unwrap();
		}
	}
	let cli = Cli::parse();

	// Update tissue box
	match cli.command {
		Some(command) => {
			tracing_subscriber::fmt::init();
			let mut tissue_box = TissueBox::open(&cli.input).unwrap_or_else(|msg| {
				error!("failed to open {}: {msg}", cli.input.display());
				exit(1);
			});

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
			if let Err(msg) = tissuebox::tui::run(&cli.input, env::current_exe().ok().as_deref()) {
				error!("{msg}");
				exit(1);
			}
		}
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	fn test_box() -> TissueBox {
		let mut tissue_box = TissueBox::default();
		tissue_box.create("Foo".into());
		tissue_box.get_mut(0).unwrap().describe("Depends on Bar implementation".into());
		tissue_box.get_mut(0).unwrap().tag("bug".into());
		tissue_box.create("Bar".into());
		tissue_box.get_mut(1).unwrap().describe("Implement using abc".into());
		tissue_box.get_mut(1).unwrap().describe("Remove xyz".into());
		tissue_box.get_mut(1).unwrap().tag("good first issue".into());
		tissue_box.get_mut(1).unwrap().tag("help wanted".into());
		tissue_box
	}

	#[test]
	fn list_all() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List { index: None, which: None });
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn list_first() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List { index: Some(0), which: None });
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn list_first_title() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List {
			index: Some(0),
			which: Some(cli::WhichList::Title),
		});
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn list_first_descriptions() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List {
			index: Some(0),
			which: Some(cli::WhichList::Description(cli::OptionIndex { index: None })),
		});
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn list_first_description() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List {
			index: Some(0),
			which: Some(cli::WhichList::Description(cli::OptionIndex { index: Some(0) })),
		});
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn list_first_tags() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List {
			index: Some(0),
			which: Some(cli::WhichList::Tags),
		});
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn filtered_list_without_index() {
		let mut tissue_box = test_box();
		let command = cli::Command::List(cli::List { index: None, which: Some(cli::WhichList::Title) });
		assert!(cli::run(command, &mut tissue_box).is_err());
	}

	#[test]
	fn add() {
		const TITLE: &str = "Baz";
		let mut tissue_box = test_box();
		let command = cli::Command::Add(cli::Add { title: TITLE.into() });
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert_eq!(tissue_box.get(2).unwrap().title, TITLE);
	}

	#[test]
	fn describe() {
		const DESC: &str = "Depends on Baz";
		let mut tissue_box = test_box();
		let command = cli::Command::Describe(cli::Describe { description: DESC.into(), index: Some(0) });
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert_eq!(tissue_box.get(0).unwrap().description.get(1).map(|x| x.as_str()), Some(DESC));
	}

	#[test]
	fn describe_last() {
		const DESC: &str = "Depends on Foo";
		let mut tissue_box = test_box();
		let command = cli::Command::Describe(cli::Describe { description: DESC.into(), index: None });
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert_eq!(tissue_box.get(1).unwrap().description.get(2).map(|x| x.as_str()), Some(DESC));
	}

	#[test]
	fn tag() {
		const TAG: &str = "good first issue";
		let mut tissue_box = test_box();
		let command = cli::Command::Tag(cli::Tag { tag: TAG.into(), index: Some(0) });
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert!(tissue_box.get(0).unwrap().tags.contains(TAG));
	}

	#[test]
	fn tag_last() {
		const TAG: &str = "bug";
		let mut tissue_box = test_box();
		let command = cli::Command::Tag(cli::Tag { tag: TAG.into(), index: None });
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert!(tissue_box.get(1).unwrap().tags.contains(TAG));
	}

	#[test]
	fn remove_tissue() {
		let mut tissue_box = test_box();
		let command = cli::Command::Remove(cli::Remove { index: 1, which: None });
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert!(tissue_box.get(1).is_none());
	}

	#[test]
	fn remove_missing_tissue() {
		let mut tissue_box = test_box();
		let command = cli::Command::Remove(cli::Remove { index: 2, which: None });
		assert!(cli::run(command, &mut tissue_box).is_err());
	}

	#[test]
	fn remove_tissue_description() {
		let mut tissue_box = test_box();
		let command = cli::Command::Remove(cli::Remove {
			index: 1,
			which: Some(cli::WhichRemove::Description(cli::Index { index: 1 })),
		});
		assert!(cli::run(command, &mut tissue_box).is_ok());
		assert!(tissue_box.get(1).unwrap().description.get(1).is_none());
	}

	#[test]
	fn remove_missing_tissue_description() {
		let mut tissue_box = test_box();
		let command = cli::Command::Remove(cli::Remove {
			index: 1,
			which: Some(cli::WhichRemove::Description(cli::Index { index: 2 })),
		});
		assert!(cli::run(command, &mut tissue_box).is_err());
	}

	#[test]
	fn remove_tissue_tag() {
		let mut tissue_box = test_box();
		let command = cli::Command::Remove(cli::Remove {
			index: 1,
			which: Some(cli::WhichRemove::Tag(cli::TagName { tag: "good first issue".into() })),
		});
		assert!(cli::run(command, &mut tissue_box).is_ok());
	}

	#[test]
	fn remove_missing_tissue_tag() {
		let mut tissue_box = test_box();
		let command = cli::Command::Remove(cli::Remove {
			index: 1,
			which: Some(cli::WhichRemove::Tag(cli::TagName { tag: "null".into() })),
		});
		assert!(cli::run(command, &mut tissue_box).is_err());
	}
}
