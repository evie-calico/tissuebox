use crate::prelude::*;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

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
	pub index: Option<usize>,
}

#[derive(Args)]
pub struct Tag {
	pub tag: String,
	/// Index of tissue to tag
	pub index: Option<usize>,
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("no tissue with index {0}")]
	TissueNotFound(usize),
	#[error("no description with index {1} on tissue {0}")]
	DescriptionNotFound(usize, usize),
	#[error("no tag named {1} on tissue {0}")]
	TagNotFound(usize, String),
	#[error("failed to commit: {0}")]
	CommitFailed(io::Error),
	#[error("failed to publish: {0}")]
	PublishFailed(io::Error),
	#[error("list command specified without index")]
	InvalidListCommand,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub fn run(command: Command, tissue_box: &mut TissueBox) -> Result<Option<String>> {
	match command {
		Command::List(List { index: None, which: None }) => Ok(Some(tissue_box.to_string())),
		Command::List(List { index: Some(index), which: None }) => Ok(Some(tissue_box.get(index).map(ToString::to_string).ok_or(Error::TissueNotFound(index))?)),
		Command::List(List {
			index: Some(index),
			which: Some(WhichList::Title),
		}) => Ok(Some(tissue_box.get(index).map(|x| x.title.clone() + "\n").ok_or(Error::TissueNotFound(index))?)),
		Command::List(List {
			index: Some(index),
			which: Some(WhichList::Description(OptionIndex { index: None })),
		}) => Ok(Some(tissue_box.get(index).map(|x| x.description.join("\n")).ok_or(Error::TissueNotFound(index))?)),
		Command::List(List {
			index: Some(tissue_index),
			which: Some(WhichList::Description(OptionIndex { index: Some(index) })),
		}) => Ok(Some(
			tissue_box
				.get(tissue_index)
				.ok_or(Error::TissueNotFound(tissue_index))?
				.description
				.get(index)
				.map(|x| x.clone() + "\n")
				.ok_or(Error::DescriptionNotFound(tissue_index, index))?,
		)),
		Command::List(List {
			index: Some(index),
			which: Some(WhichList::Tags),
		}) => {
			let tissue = tissue_box.get(index).ok_or(Error::TissueNotFound(index))?;
			let mut iter = tissue.tags.iter();
			let mut tags = iter.next().cloned().unwrap_or_default();
			for next in iter {
				tags.push_str(", ");
				tags.push_str(next);
			}
			tags.push('\n');
			Ok(Some(tags))
		}
		Command::List(List { index: None, which: Some(_) }) => Err(Error::InvalidListCommand),
		Command::Add(Add { title }) => {
			tissue_box.create(title);
			Ok(None)
		}
		Command::Describe(Describe { index, description }) => {
			let index = index.unwrap_or(tissue_box.tissues.len() - 1);
			tissue_box.get_mut(index).ok_or(Error::TissueNotFound(index))?.describe(description);
			Ok(None)
		}
		Command::Tag(Tag { index, tag }) => {
			let index = index.unwrap_or(tissue_box.tissues.len() - 1);
			tissue_box.get_mut(index).ok_or(Error::TissueNotFound(index))?.tag(tag);
			Ok(None)
		}
		Command::Remove(Remove { index, which: None }) => {
			tissue_box.remove(index).ok_or(Error::TissueNotFound(index))?;
			Ok(None)
		}
		Command::Remove(Remove {
			index: tissue_index,
			which: Some(WhichRemove::Description(Index { index })),
		}) => {
			let tissue = tissue_box.get_mut(tissue_index).ok_or(Error::TissueNotFound(tissue_index))?;
			tissue.description.get(index).ok_or(Error::DescriptionNotFound(tissue_index, index))?;
			tissue.description.remove(index);
			Ok(None)
		}
		Command::Remove(Remove {
			index,
			which: Some(WhichRemove::Tag(TagName { tag })),
		}) => {
			if tissue_box.get_mut(index).ok_or(Error::TissueNotFound(index))?.tags.remove(&tag) {
				Ok(None)
			} else {
				Err(Error::TagNotFound(index, tag))
			}
		}
		Command::Commit(Index { index }) => {
			tissue_box.get_mut(index).ok_or(Error::TissueNotFound(index))?.commit().map_err(Error::CommitFailed)?;
			tissue_box.remove(index).expect("index used by get_mut");
			Ok(None)
		}
		Command::Publish(Index { index }) => {
			tissue_box.get_mut(index).ok_or(Error::TissueNotFound(index))?.publish().map_err(Error::PublishFailed)?;
			tissue_box.remove(index).expect("index used by get_mut");
			Ok(None)
		}
	}
}
