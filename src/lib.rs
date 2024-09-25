pub mod cli;
pub mod tui;

pub mod prelude {
	pub use super::*;
	pub use cli::Cli;
}

use std::{collections::HashSet, fs, io, path::Path};

pub const DAEMONIZE_ARG: &str = "__internal_daemonize";

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tissue {
	pub title: String,
	#[serde(default)]
	pub description: Vec<String>,
	#[serde(default)]
	pub tags: HashSet<String>,
}

impl Tissue {
	pub fn describe(&mut self, description: String) {
		self.description.push(description);
	}

	pub fn tag(&mut self, tag: String) {
		self.tags.insert(tag);
	}

	pub fn publish(&self) -> io::Result<()> {
		let output = std::process::Command::new("gh").args(["label", "list"]).output()?;
		if output.status.success() {
			let labels = String::from_utf8_lossy(&output.stdout);
			let labels = labels.lines().map(|s| s.split_once('\t').unwrap_or_default().0).collect::<Vec<_>>();
			for tag in &self.tags {
				if !labels.contains(&tag.as_str()) {
					let output = std::process::Command::new("gh").args(["label", "create", tag]).output()?;
					if !output.status.success() {
						return Err(io::Error::other(String::from_utf8_lossy(&output.stderr)));
					}
				}
			}
		} else {
			return Err(io::Error::other(String::from_utf8_lossy(&output.stderr)));
		}

		let output = std::process::Command::new("gh")
			.args(["issue", "create"])
			.args(["--title", &self.title])
			.args(["--body", &self.description.join("\n")])
			.args(["--label", &self.tags.iter().fold(String::new(), |a, b| a + "\n" + b)])
			.output()?;
		if output.status.success() {
			Ok(())
		} else {
			Err(io::Error::other(String::from_utf8_lossy(&output.stderr)))
		}
	}

	pub fn commit(&self) -> io::Result<()> {
		let output = std::process::Command::new("git").arg("add").arg("--all").output()?;
		if output.status.success() {
			let output = std::process::Command::new("git").arg("commit").arg("-m").arg(&self.title).output()?;
			if output.status.success() {
				Ok(())
			} else {
				Err(io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()))
			}
		} else {
			Err(io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()))
		}
	}
}

impl std::fmt::Display for Tissue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let Tissue { title, description, tags } = self;
		write!(f, "{title}")?;
		if !tags.is_empty() {
			let tags = tags.iter().cloned().collect::<Vec<String>>().join(", ");
			write!(f, " ({tags})",)?;
		}
		writeln!(f)?;
		for description in description {
			writeln!(f, "  - {description}")?;
		}
		Ok(())
	}
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TissueBox {
	#[serde(default)]
	recycle_bin: Vec<Tissue>,
	#[serde(default)]
	tissues: Vec<Tissue>,
	#[serde(default)]
	starred: Option<usize>,
}

impl TissueBox {
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		toml::from_str(&fs::read_to_string(path.as_ref())?).map_err(io::Error::other)
	}

	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		fs::write(path.as_ref(), toml::to_string(self).map_err(io::Error::other)?)
	}

	pub fn create(&mut self, title: String) {
		self.tissues.push(Tissue { title, ..Default::default() })
	}

	#[must_use]
	pub fn remove(&mut self, index: usize) -> Option<Tissue> {
		// If this issue is starred, reset the star state.
		if let Some(i) = self.starred {
			if i == index {
				self.starred = None;
			}
		}
		self.tissues.get(index)?;
		let tissue = self.tissues.remove(index);
		self.recycle_bin.push(tissue.clone());
		Some(tissue)
	}

	pub fn restore(&mut self, index: usize) -> Option<&Tissue> {
		self.recycle_bin.get(index)?;
		self.tissues.push(self.recycle_bin.remove(index));
		self.tissues.last()
	}

	pub fn get(&self, index: usize) -> Option<&Tissue> {
		self.tissues.get(index)
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut Tissue> {
		self.tissues.get_mut(index)
	}
}

impl std::fmt::Display for TissueBox {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (index, tissue) in self.tissues.iter().enumerate() {
			write!(f, "{index}. {tissue}")?;
		}
		Ok(())
	}
}
