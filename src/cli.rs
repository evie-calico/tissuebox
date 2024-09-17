use crate::prelude::*;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::process::exit;
use tracing::error;

#[derive(Parser)]
pub struct Cli {
	#[clap(short, long, default_value = ".tissuebox")]
	pub input: PathBuf,
	#[command(subcommand)]
	pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
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
pub struct Index {
	pub index: usize,
}

#[derive(Args)]
pub struct OptionIndex {
	pub index: Option<usize>,
}

#[derive(Args)]
pub struct List {
	pub index: Option<usize>,
	#[command(subcommand)]
	pub which: Option<WhichList>,
}

#[derive(Subcommand)]
pub enum WhichList {
	/// List title
	Title,
	/// List descriptions
	Description(OptionIndex),
	/// List all tags
	Tags,
}

#[derive(Args)]
pub struct Add {
	/// Title of the new issue.
	///
	/// This should be formatted as a prospective git commit or issue title.
	pub title: String,
}

#[derive(Args)]
pub struct Describe {
	pub description: String,
	/// Index of tissue to describe
	pub index: usize,
}

#[derive(Args)]
pub struct Tag {
	pub tag: String,
	/// Index of tissue to tag
	pub index: usize,
}

#[derive(Args)]
pub struct Remove {
	/// Which tissue to delete
	pub index: usize,
	/// Remove a single field, instead of the whole tissue
	#[command(subcommand)]
	pub which: Option<WhichRemove>,
}

#[derive(Subcommand)]
pub enum WhichRemove {
	/// Remove a description
	Description(Index),
	/// Remove a tag
	Tag(TagName),
}

#[derive(Args)]
pub struct TagName {
	pub tag: String,
}

pub fn run(command: Command, tissue_box: &mut TissueBox) {
	fn try_get(tissue_box: &TissueBox, index: usize) -> &Tissue {
		let Some(tissue) = tissue_box.get(index) else {
			error!("no tissue with index {index}");
			exit(1);
		};
		tissue
	}

	fn try_get_mut(tissue_box: &mut TissueBox, index: usize) -> &mut Tissue {
		let Some(tissue) = tissue_box.get_mut(index) else {
			error!("no tissue with index {index}");
			exit(1);
		};
		tissue
	}

	match command {
		Command::List(List { index: None, which: None }) => print!("{tissue_box}"),
		Command::List(List { index: Some(index), which: None }) => print!("{}", try_get(tissue_box, index)),
		Command::List(List {
			index: Some(index),
			which: Some(WhichList::Title),
		}) => {
			println!("{}", try_get(tissue_box, index).title);
		}
		Command::List(List {
			index: Some(index),
			which: Some(WhichList::Description(OptionIndex { index: None })),
		}) => {
			println!("{}", try_get(tissue_box, index).description.join("\n"));
		}
		Command::List(List {
			index: Some(tissue_index),
			which: Some(WhichList::Description(OptionIndex { index: Some(index) })),
		}) => {
			println!(
				"{}",
				try_get(tissue_box, tissue_index).description.get(index).unwrap_or_else(|| {
					error!("no description with index {index} on tissue {index}");
					exit(1);
				})
			)
		}
		Command::List(List {
			index: Some(index),
			which: Some(WhichList::Tags),
		}) => {
			let tissue = try_get(tissue_box, index);
			let mut iter = tissue.tags.iter();
			if let Some(first) = iter.next() {
				print!("{first}");
				for next in iter {
					print!(", {next}");
				}
				println!();
			}
		}
		Command::List(List { index: None, which: Some(_) }) => panic!("list subcommand specified without index"),
		Command::Add(Add { title }) => {
			tissue_box.create(title);
		}
		Command::Describe(Describe { index, description }) => {
			try_get_mut(tissue_box, index).describe(description);
		}
		Command::Tag(Tag { index, tag }) => {
			try_get_mut(tissue_box, index).tag(tag);
		}
		Command::Remove(Remove { index, which: None }) => {
			if tissue_box.remove(index).is_none() {
				error!("no tissue with index {index}");
				exit(1);
			};
		}
		Command::Remove(Remove {
			index: tissue_index,
			which: Some(WhichRemove::Description(Index { index })),
		}) => {
			let tissue = try_get_mut(tissue_box, tissue_index);
			if tissue.description.get(index).is_none() {
				error!("no description with index {index} on tissue {index}");
				exit(1);
			}
			tissue.description.remove(index);
		}
		Command::Remove(Remove {
			index,
			which: Some(WhichRemove::Tag(TagName { tag })),
		}) => {
			if !try_get_mut(tissue_box, index).tags.remove(&tag) {
				error!("no tag named {tag}");
				exit(1);
			}
		}
		Command::Commit(Index { index }) => {
			try_get_mut(tissue_box, index).commit().unwrap_or_else(|msg| {
				error!("failed to commit: {msg}");
				exit(1);
			});
		}
		Command::Publish(Index { index }) => {
			try_get_mut(tissue_box, index).publish().unwrap_or_else(|msg| {
				error!("failed to publish: {msg}");
				exit(1);
			});
		}
	}
}
