use std::{collections::HashSet, fs, io, path::Path};

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
		let output = std::process::Command::new("gh")
			.args(["label", "list"])
			.output()?;
		if output.status.success() {
			let labels = String::from_utf8_lossy(&output.stdout);
			let labels = labels
				.lines()
				.map(|s| s.split_once('\t').unwrap_or_default().0)
				.collect::<Vec<_>>();
			for tag in &self.tags {
				if !labels.contains(&tag.as_str()) {
					let output = std::process::Command::new("gh")
						.args(["label", "create", tag])
						.output()?;
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
			.args([
				"--label",
				&self.tags.iter().fold(String::new(), |a, b| a + "\n" + b),
			])
			.output()?;
		if output.status.success() {
			Ok(())
		} else {
			Err(io::Error::other(String::from_utf8_lossy(&output.stderr)))
		}
	}

	pub fn commit(&self) -> io::Result<()> {
		let output = std::process::Command::new("git")
			.arg("add")
			.arg("--all")
			.output()?;
		if output.status.success() {
			let output = std::process::Command::new("git")
				.arg("commit")
				.arg("-m")
				.arg(&self.title)
				.output()?;
			if output.status.success() {
				Ok(())
			} else {
				Err(io::Error::other(
					String::from_utf8_lossy(&output.stderr).to_string(),
				))
			}
		} else {
			Err(io::Error::other(
				String::from_utf8_lossy(&output.stderr).to_string(),
			))
		}
	}
}

impl std::fmt::Display for Tissue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let Tissue {
			title,
			description,
			tags,
		} = self;
		write!(f, "{title}")?;
		if !tags.is_empty() {
			write!(
				f,
				" ({})",
				tags.iter().cloned().collect::<Vec<String>>().join(", ")
			)?;
		}
		writeln!(f)?;
		for description in description {
			writeln!(f, "  - {description}")?;
		}
		Ok(())
	}
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Box {
	#[serde(default)]
	tissues: Vec<Tissue>,
}

impl Box {
	pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
		toml::from_str(&fs::read_to_string(path.as_ref())?).map_err(io::Error::other)
	}

	pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
		fs::write(
			path.as_ref(),
			toml::to_string(self).map_err(io::Error::other)?,
		)
	}

	pub fn create(&mut self, title: String) {
		self.tissues.push(Tissue {
			title,
			..Default::default()
		})
	}

	#[must_use]
	pub fn remove(&mut self, index: usize) -> Option<Tissue> {
		self.tissues.get(index)?;
		Some(self.tissues.remove(index))
	}

	pub fn get(&self, index: usize) -> Option<&Tissue> {
		self.tissues.get(index)
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut Tissue> {
		self.tissues.get_mut(index)
	}
}

impl std::fmt::Display for Box {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (index, tissue) in self.tissues.iter().enumerate() {
			write!(f, "{index}. {tissue}")?;
		}
		Ok(())
	}
}

pub mod tui {
	use crossterm::event::{self, KeyCode, KeyEventKind};
	use ratatui::{
		layout::{Alignment, Offset, Rect},
		style::Stylize,
		symbols::border,
		text::{Line, Text},
		widgets::{
			block::{Position, Title},
			Block, Padding, Paragraph,
		},
		DefaultTerminal,
	};
	use std::{io, path::Path};

	pub fn run(tissue_box: &mut crate::Box, save_path: &Path) -> io::Result<()> {
		let mut terminal = ratatui::init();
		terminal.clear()?;
		let result = tui(terminal, tissue_box, save_path);
		ratatui::restore();
		result
	}

	fn tui(
		mut terminal: DefaultTerminal,
		tissue_box: &mut crate::Box,
		save_path: &Path,
	) -> io::Result<()> {
		fn gather_line(line: &mut String, code: KeyCode) -> bool {
			match code {
				KeyCode::Backspace => {
					line.pop();
				}
				KeyCode::Enter => return true,
				KeyCode::Char(c) => line.push(c),
				_ => {}
			}
			false
		}

		enum Mode {
			Normal,
			Help,
			Add(String),
			Describe(String),
			Tag(String),
			Copy,
			Publish,
			FailedPublish(String),
			Commit,
			FailedCommit(String),
			Remove,
			RemoveDescription(usize),
			RemoveTag(String),
		}
		let mut index = 0usize;
		let mut mode = Mode::Normal;
		loop {
			index = index.min(tissue_box.tissues.len() - 1);
			terminal.draw(|frame| {
				let area = frame.area();
				let title = Title::from(" Tissue Box ".red().bold());
				let instructions = match &mode {
					Mode::Normal => {
						Title::from(Line::from(vec![
							" H".red().bold(),
							"elp".into(),
							" a".red().bold(),
							"dd".into(),
							" d".red().bold(),
							"escribe".into(),
							" t".red().bold(),
							"ag".into(),
							" r".red().bold(),
							"emove".into(),
							" q".red().bold(),
							"uit ".into(),
						]))
					},
					Mode::Help => Title::from(Line::from(vec![
						" Help! ".blue().bold(),
					])),
					Mode::Add(title) => Title::from(Line::from(vec![
						" Add tissue: ".blue().bold(),
						title.into(),
						"_ ".into(),
					])),
					Mode::Describe(description) => Title::from(Line::from(vec![
						" Describe tissue: ".blue().bold(),
						description.into(),
						"_ ".into(),
					])),
					Mode::Tag(tag) => Title::from(Line::from(vec![
						" Tag tissue: ".blue().bold(),
						tag.into(),
						"_ ".into(),
					])),
					Mode::Copy => {
						Title::from(Line::from(vec![
							" Copy what?:".blue().bold(),
							" t".red().bold(),
							"itle".into(),
							" d".red().bold(),
							"escription".into(),
							" l".red().bold(),
							"ist ".into(),
						]))
					}
					Mode::Publish => {
						Title::from(Line::from(vec![
							" Really Publish?:".blue().bold(),
							" y".red().bold(),
							"es".into(),
							" N".red().bold(),
							"o ".into(),
						]))
					}
					Mode::FailedPublish(_) => {
						Title::from(Line::from(vec![
							" Publish Failed ".blue().bold(),
						]))
					}
					Mode::Commit => {
						Title::from(Line::from(vec![
							" Really Commit?:".blue().bold(),
							" y".red().bold(),
							"es".into(),
							" N".red().bold(),
							"o ".into(),
						]))
					}
					Mode::FailedCommit(_) => {
						Title::from(Line::from(vec![
							" Commit Failed ".blue().bold(),
						]))
					}
					Mode::Remove => {
						Title::from(Line::from(vec![
							" Remove what?:".blue().bold(),
							" T".red().bold(),
							"issue".into(),
							" d".red().bold(),
							"escription".into(),
							" t".red().bold(),
							"ag ".into(),
						]))
					}
					Mode::RemoveDescription(_) => Title::from(Line::from(vec![
						" Remove which description? ".blue().bold(),
					])),
					Mode::RemoveTag(tag) => Title::from(Line::from(vec![
						" Remove tag: ".blue().bold(),
						tag.into(),
						"_ ".into(),
					])),
				};
				let block = Block::bordered()
					.title(title.alignment(Alignment::Center))
					.title(
						instructions
							.alignment(Alignment::Center)
							.position(Position::Bottom),
					)
					.padding(Padding::horizontal(2))
					.border_set(border::ROUNDED);

				let paper = Paragraph::new(
					"  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\n ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\n▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\n▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\n",
				)
				.centered();
				frame.render_widget(paper, Rect { height: 4, ..area });
				let mut body = Text::default();
				match &mode {
					Mode::FailedPublish(reason) | Mode::FailedCommit(reason) => body.lines.push(reason.clone().into()),
					Mode::Help => {
						body.lines.push("Welcome to tissuebox!".blue().into());
						body.lines.push("".into());
						body.lines.push("Basic Commands".red().into());
						body.lines.push(" a (add): Create a new tissue under the given name".into());
						body.lines.push(" d (describe): Append a description to the selected tissue".into());
						body.lines.push(" t (tag): Assign a tag to the selected tissue".into());
						body.lines.push(" r (remove): Delete the selected issue".into());
						body.lines.push("".into());
						body.lines.push("Output commands".red().into());
						body.lines.push(" c (copy): Copy the title or description of the selected tissue to the clipboard".into());
						body.lines.push(" C (commit): Add all files to the git index and commit.".into());
						body.lines.push("             Uses the selected tissue's title as the message".into());
						body.lines.push("             Equivalent to `git add --all && git commit -m {title}`".into());
						body.lines.push(" P (publish): Publish the selected issue to GitHub. Requires the `gh` command.".into());
					}
					_ => {
						for (i, tissue) in tissue_box.tissues.iter().enumerate() {
							let mut title: Line = tissue.title.clone().into();
							if let Mode::RemoveDescription(_) = mode {} else if i == index{
								title = title.black().on_white();
							};
							for tag in &tissue.tags {
								title.spans.push(format!(" ({tag})").magenta());
							}
							body.lines.push(title);
							for (i, description) in tissue.description.iter().enumerate() {
								if let Mode::RemoveDescription(index) = mode {
									if index == i {
										body.lines.push(format!("- {description}").black().on_white().into());
										continue;
									}
								}
								body.lines.push(format!("- {description}").dark_gray().into());
							}
						}
					}
				}
				frame.render_widget(Paragraph::new(body).block(block), area.offset(Offset { x: 0, y: 4 }));
			})?;

			if let event::Event::Key(key) = event::read()? {
				if key.kind == KeyEventKind::Press {
					if key.code == KeyCode::Esc {
						mode = Mode::Normal;
					} else {
						mode = match mode {
							Mode::Normal => match key.code {
								KeyCode::Char('k')
								| KeyCode::Char('h')
								| KeyCode::Up
								| KeyCode::Left => {
									index = index.saturating_sub(1);
									Mode::Normal
								}
								KeyCode::Char('j')
								| KeyCode::Char('l')
								| KeyCode::Down
								| KeyCode::Right => {
									index += 1;
									Mode::Normal
								}
								KeyCode::Char('H') => Mode::Help,
								KeyCode::Char('a') => Mode::Add(String::new()),
								KeyCode::Char('d') => Mode::Describe(String::new()),
								KeyCode::Char('t') => Mode::Tag(String::new()),
								KeyCode::Char('c') => Mode::Copy,
								KeyCode::Char('C') => Mode::Commit,
								KeyCode::Char('P') => Mode::Publish,
								KeyCode::Char('r') => Mode::Remove,
								KeyCode::Char('q') => return Ok(()),
								KeyCode::Char('Q') => {
									let _ = crossterm::terminal::disable_raw_mode();
									let _ = crossterm::execute!(
										std::io::stdout(),
										crossterm::terminal::LeaveAlternateScreen
									);
									std::process::exit(0);
								}
								_ => Mode::Normal,
							},
							m @ Mode::Help
							| m @ Mode::FailedPublish(_)
							| m @ Mode::FailedCommit(_) => {
								if let KeyCode::Char(_) = key.code {
									Mode::Normal
								} else {
									m
								}
							}
							Mode::Add(mut title) => {
								if gather_line(&mut title, key.code) {
									tissue_box.create(title);
									tissue_box.save(save_path);
									Mode::Normal
								} else {
									Mode::Add(title)
								}
							}
							Mode::Describe(mut description) => {
								if gather_line(&mut description, key.code) {
									tissue_box.tissues[index].describe(description);
									tissue_box.save(save_path);
									Mode::Normal
								} else {
									Mode::Describe(description)
								}
							}
							Mode::Tag(mut tag) => {
								if gather_line(&mut tag, key.code) {
									tissue_box.tissues[index].tag(tag);
									tissue_box.save(save_path);
									Mode::Normal
								} else {
									Mode::Tag(tag)
								}
							}
							Mode::Copy => match key.code {
								KeyCode::Char('t') => todo!(),
								KeyCode::Char('d') => todo!(),
								KeyCode::Char('l') => todo!(),
								_ => Mode::Copy,
							},
							Mode::Publish => match key.code {
								KeyCode::Char('y') | KeyCode::Char('Y') => {
									let tissue = &tissue_box.tissues[index];
									match tissue.publish() {
										Ok(()) => {
											let _ = tissue_box.remove(index);
											tissue_box.save(save_path);
											Mode::Normal
										}
										Err(msg) => Mode::FailedPublish(msg.to_string()),
									}
								}
								KeyCode::Char('n') | KeyCode::Char('N') => Mode::Normal,
								_ => Mode::Publish,
							},
							Mode::Commit => match key.code {
								KeyCode::Char('y') | KeyCode::Char('Y') => {
									match tissue_box.tissues[index].commit() {
										Ok(()) => {
											let _ = tissue_box.remove(index);
											tissue_box.save(save_path);
											Mode::Normal
										}
										Err(msg) => Mode::FailedCommit(msg.to_string()),
									}
								}
								KeyCode::Char('n') | KeyCode::Char('N') => Mode::Normal,
								_ => Mode::Commit,
							},
							Mode::Remove => match key.code {
								KeyCode::Char('T') => {
									tissue_box.tissues.remove(index);
									tissue_box.save(save_path);
									Mode::Normal
								}
								KeyCode::Char('d') => {
									if tissue_box.tissues[index].description.is_empty() {
										Mode::Normal
									} else {
										Mode::RemoveDescription(0)
									}
								}
								KeyCode::Char('t') => Mode::RemoveTag(String::new()),
								_ => Mode::Remove,
							},
							Mode::RemoveDescription(i) => {
								let tissue = &mut tissue_box.tissues[index];
								match key.code {
									KeyCode::Char('k')
									| KeyCode::Char('h')
									| KeyCode::Up
									| KeyCode::Left => Mode::RemoveDescription(i.saturating_sub(1)),
									KeyCode::Char('j')
									| KeyCode::Char('l')
									| KeyCode::Down
									| KeyCode::Right => Mode::RemoveDescription(
										(i + 1).min(tissue.description.len() - 1),
									),
									KeyCode::Enter => {
										tissue.description.remove(i);
										tissue_box.save(save_path);
										Mode::Normal
									}
									_ => Mode::RemoveDescription(i),
								}
							}
							Mode::RemoveTag(mut tag) => {
								if gather_line(&mut tag, key.code) {
									tissue_box.tissues[index].tags.remove(&tag);
									tissue_box.save(save_path);
									Mode::Normal
								} else {
									Mode::RemoveTag(tag)
								}
							}
						}
					}
				}
			}
		}
	}
}
