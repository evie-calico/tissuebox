use crate::prelude::*;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
	layout::{Alignment, Rect},
	style::Stylize,
	symbols::border,
	text::{Line, Span, Text},
	widgets::{
		block::{Position, Title},
		Block, Padding, Paragraph,
	},
	DefaultTerminal,
};
use std::{
	io::{self, Write},
	path::Path,
	process,
};

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error(transparent)]
	Io(#[from] io::Error),
	#[error("clipboard is not available")]
	MissingClipboard,
	#[error(transparent)]
	Arboard(#[from] arboard::Error),
}

enum Mode {
	Normal,
	Help,
	Add(String),
	Describe(String),
	Tag(String),
	Edit(String),
	Copy,
	Publish,
	Commit,
	Remove,
	RemoveDescription(usize),
	RemoveTag(String),
	Restore(usize),
}

pub fn run(path: &Path, clipboard_daemon: Option<&Path>) -> io::Result<()> {
	let mut terminal = ratatui::init();
	terminal.clear()?;
	let result = tui(terminal, path, clipboard_daemon);
	ratatui::restore();
	result
}

fn tui(mut terminal: DefaultTerminal, path: &Path, clipboard_daemon: Option<&Path>) -> io::Result<()> {
	// If this is the first run and a git repository is present, offer to initialize .git/info/exclude.
	let init_git_exclude = if !path.try_exists()? {
		let response = if Path::new(".git").try_exists()? {
			'git_prompt: loop {
				terminal.draw(|frame| {
					let instructions = Title::from(vec![" y".red().bold(), "es ".into(), "n".red().bold(), "o ".into()]);
					let block = Block::bordered().title(instructions.alignment(Alignment::Center).position(Position::Bottom)).padding(Padding::horizontal(2)).border_set(border::ROUNDED);
					let area = frame.area();
					let mut body = Text::default();
					let path_str = path.to_string_lossy();
					body.lines.push(Line::from(vec!["Tissuebox will initialize the file \"".into(), path_str.blue(), "\".".into()]));
					body.lines.push(Line::default());
					body.lines.push("Would you like to exclude it from git?".into());
					body.lines.push(Line::from(vec!["Note: This will update ".into(), ".git/info/exclude".blue(), ", not the public ".into(), ".gitignore".blue()]));

					frame.render_widget(Paragraph::new(body).block(block).centered(), area);
				})?;

				if let event::Event::Key(key) = event::read()? {
					if key.kind == KeyEventKind::Press {
						match key.code {
							KeyCode::Char('y') | KeyCode::Char('Y') => {
								break 'git_prompt true;
							}
							KeyCode::Char('n') | KeyCode::Char('N') => {
								break 'git_prompt true;
							}
							_ => {}
						}
					}
				}
			}
		} else {
			false
		};
		fs::write(path, [])?;
		response
	} else {
		false
	};
	let mut tissue_box = TissueBox::open(path)?;
	if init_git_exclude {
		let mut git_exclude = fs::OpenOptions::new().append(true).open(".git/info/exclude")?;
		git_exclude.write_all("\n# Created by tissuebox\n".as_bytes())?;
		git_exclude.write_all(path.as_os_str().as_encoded_bytes())?;
		git_exclude.write_all("\n".as_bytes())?;
	}

	let mut index = 0;
	let mut mode = Mode::Normal;
	let mut last_error: Result<(), Error> = Ok(());
	loop {
		index = index.min(tissue_box.tissues.len().saturating_sub(1));
		terminal.draw(|frame| {
			let area = frame.area();

			// Paper
			frame.render_widget(
				Paragraph::new(concat! {
					" ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\n",
					" ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓\n",
					"▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ \n",
					"▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ \n",
				})
				.centered(),
				Rect { height: 4, ..area },
			);

			// TissueBox
			let title = Title::from(" tissuebox ".red().bold());
			let instructions = instructions(&mode);
			let block = Block::bordered()
				.title(title.alignment(Alignment::Center))
				.title(instructions.alignment(Alignment::Center).position(Position::Bottom))
				.padding(Padding::horizontal(2))
				.border_set(border::ROUNDED);

			let mut body = Text::default();
			match &mode {
				Mode::Help => {
					help(&mut body);
				}
				Mode::Restore(index) => {
					format_tissues(&mut body, &tissue_box.recycle_bin, *index, None, None);
				}
				Mode::RemoveDescription(description_index) => {
					format_tissues(&mut body, &tissue_box.tissues, index, tissue_box.starred, Some(*description_index));
				}
				_ => {
					format_tissues(&mut body, &tissue_box.tissues, index, tissue_box.starred, None);
				}
			}
			let paragraph_area = Rect { y: area.y + 4, height: area.height - 5, ..area };
			frame.render_widget(Paragraph::new(body).block(block).scroll(((sum_lines(&tissue_box.tissues, index) as u16).saturating_sub(paragraph_area.height / 2 - 1), 0)), paragraph_area);

			// Errors
			if let Err(msg) = &last_error {
				frame.render_widget(Paragraph::new(msg.to_string().red()), Rect { y: area.y + area.height - 1, height: 1, ..area });
			}
		})?;

		if let event::Event::Key(key) = event::read()? {
			if key.kind == KeyEventKind::Press {
				if key.code == KeyCode::Esc {
					mode = Mode::Normal;
				}
				if let (Mode::Normal, KeyCode::Char('q')) = (&mode, key.code) {
					return Ok(());
				} else {
					mode = match input(mode, key.code, &mut index, &mut tissue_box) {
						InputResult::Mode(mode) => mode,
						InputResult::Copy(text) => {
							if let Some(clipboard_daemon) = clipboard_daemon {
								last_error = process::Command::new(clipboard_daemon)
									.args([DAEMONIZE_ARG, &text])
									.stdin(process::Stdio::null())
									.stdout(process::Stdio::null())
									.stderr(process::Stdio::null())
									.current_dir("/")
									.spawn()
									.map(|_| ())
									.map_err(Into::into);
								Mode::Normal
							} else {
								last_error = Err(Error::MissingClipboard);
								Mode::Normal
							}
						}
						InputResult::Error(error) => {
							last_error = error;
							Mode::Normal
						}
						InputResult::Changed => {
							last_error = tissue_box.save(path).map_err(Error::from);
							Mode::Normal
						}
					}
				}
			}
		}
	}
}

enum InputResult {
	Mode(Mode),
	Copy(String),
	Error(Result<(), Error>),
	Changed,
}

impl From<Mode> for InputResult {
	fn from(mode: Mode) -> Self {
		Self::Mode(mode)
	}
}

impl<T: Into<Error>> From<T> for InputResult {
	fn from(error: T) -> Self {
		Self::Error(Err(error.into()))
	}
}

impl<T: Into<Error>> From<Result<(), T>> for InputResult {
	fn from(error: Result<(), T>) -> Self {
		Self::Error(error.map_err(Into::into))
	}
}

fn input(mode: Mode, code: KeyCode, index: &mut usize, tissue_box: &mut TissueBox) -> InputResult {
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

	match mode {
		Mode::Normal => match code {
			KeyCode::Char('k') | KeyCode::Char('h') | KeyCode::Up | KeyCode::Left => {
				*index = index.saturating_sub(1);
				Mode::Normal.into()
			}
			KeyCode::Char('j') | KeyCode::Char('l') | KeyCode::Down | KeyCode::Right => {
				*index += 1;
				Mode::Normal.into()
			}
			KeyCode::Char('H') => Mode::Help.into(),
			KeyCode::Char('a') => Mode::Add(String::new()).into(),
			KeyCode::Char('R') => {
				if tissue_box.recycle_bin.is_empty() {
					Mode::Normal.into()
				} else {
					Mode::Restore(0).into()
				}
			}
			KeyCode::Char('d') if !tissue_box.tissues.is_empty() => Mode::Describe(String::new()).into(),
			KeyCode::Char('t') if !tissue_box.tissues.is_empty() => Mode::Tag(String::new()).into(),
			KeyCode::Char('e') if !tissue_box.tissues.is_empty() => Mode::Edit(String::new()).into(),
			KeyCode::Char('c') if !tissue_box.tissues.is_empty() => Mode::Copy.into(),
			KeyCode::Char('C') if !tissue_box.tissues.is_empty() => Mode::Commit.into(),
			KeyCode::Char('P') if !tissue_box.tissues.is_empty() => Mode::Publish.into(),
			KeyCode::Char('r') if !tissue_box.tissues.is_empty() => Mode::Remove.into(),
			KeyCode::Char('*') if !tissue_box.tissues.is_empty() => {
				if let Some(starred) = tissue_box.starred {
					if starred == *index {
						tissue_box.starred = None;
					} else {
						*index = starred
					}
				} else {
					tissue_box.starred = Some(*index);
				}
				InputResult::Changed
			}
			_ => Mode::Normal.into(),
		},
		m @ Mode::Help => {
			if let KeyCode::Char(_) = code {
				Mode::Normal.into()
			} else {
				m.into()
			}
		}
		Mode::Add(mut title) => {
			if gather_line(&mut title, code) {
				tissue_box.create(title);
				InputResult::Changed
			} else {
				Mode::Add(title).into()
			}
		}
		Mode::Describe(mut description) => {
			if gather_line(&mut description, code) {
				tissue_box.tissues[*index].describe(description);
				InputResult::Changed
			} else {
				Mode::Describe(description).into()
			}
		}
		Mode::Tag(mut tag) => {
			if gather_line(&mut tag, code) {
				tissue_box.tissues[*index].tag(tag);
				InputResult::Changed
			} else {
				Mode::Tag(tag).into()
			}
		}
		Mode::Edit(mut title) => {
			if gather_line(&mut title, code) {
				tissue_box.tissues[*index].title = title;
				InputResult::Changed
			} else {
				Mode::Edit(title).into()
			}
		}
		Mode::Copy => match code {
			KeyCode::Char('t') => InputResult::Copy(tissue_box.tissues[*index].title.clone()),
			KeyCode::Char('d') => InputResult::Copy(tissue_box.tissues[*index].description.join("\n")),
			KeyCode::Char('l') => InputResult::Copy(tissue_box.to_string()),
			_ => Mode::Copy.into(),
		},
		Mode::Publish => match code {
			KeyCode::Char('y') | KeyCode::Char('Y') => {
				let tissue = &tissue_box.tissues[*index];
				match tissue.publish() {
					Ok(()) => {
						let _ = tissue_box.remove(*index);
						InputResult::Changed
					}
					Err(msg) => msg.into(),
				}
			}
			KeyCode::Char('n') | KeyCode::Char('N') => Mode::Normal.into(),
			_ => Mode::Publish.into(),
		},
		Mode::Commit => match code {
			KeyCode::Char('y') | KeyCode::Char('Y') => {
				let tissue = &tissue_box.tissues[*index];
				match tissue.commit() {
					Ok(()) => {
						let _ = tissue_box.remove(*index);
						InputResult::Changed
					}
					Err(msg) => msg.into(),
				}
			}
			KeyCode::Char('n') | KeyCode::Char('N') => Mode::Normal.into(),
			_ => Mode::Commit.into(),
		},
		Mode::Remove => match code {
			KeyCode::Char('T') => {
				let _ = tissue_box.remove(*index);
				InputResult::Changed
			}
			KeyCode::Char('d') => {
				if tissue_box.tissues[*index].description.is_empty() {
					Mode::Normal.into()
				} else {
					Mode::RemoveDescription(0).into()
				}
			}
			KeyCode::Char('t') => Mode::RemoveTag(String::new()).into(),
			_ => Mode::Remove.into(),
		},
		Mode::RemoveDescription(i) => {
			let tissue = &mut tissue_box.tissues[*index];
			match code {
				KeyCode::Char('k') | KeyCode::Char('h') | KeyCode::Up | KeyCode::Left => Mode::RemoveDescription(i.saturating_sub(1)).into(),
				KeyCode::Char('j') | KeyCode::Char('l') | KeyCode::Down | KeyCode::Right => Mode::RemoveDescription((i + 1).min(tissue.description.len() - 1)).into(),
				KeyCode::Enter => {
					tissue.description.remove(i);
					InputResult::Changed
				}
				_ => Mode::RemoveDescription(i).into(),
			}
		}
		Mode::RemoveTag(mut tag) => {
			if gather_line(&mut tag, code) {
				tissue_box.tissues[*index].tags.remove(&tag);
				InputResult::Changed
			} else {
				Mode::RemoveTag(tag).into()
			}
		}
		Mode::Restore(index) => match code {
			KeyCode::Char('k') | KeyCode::Char('h') | KeyCode::Up | KeyCode::Left => Mode::Restore(index.saturating_sub(1)).into(),
			KeyCode::Char('j') | KeyCode::Char('l') | KeyCode::Down | KeyCode::Right => Mode::Restore((index + 1).min(tissue_box.recycle_bin.len() - 1)).into(),
			KeyCode::Enter => {
				tissue_box.restore(index);
				InputResult::Changed
			}
			_ => Mode::Restore(index).into(),
		},
	}
}

fn sum_lines(tissues: &[Tissue], index: usize) -> usize {
	tissues.iter().take(index).fold(0, |a, b| a + 1 + b.description.len())
}

fn format_tissues(body: &mut Text, tissues: &[Tissue], index: usize, starred: Option<usize>, description_index: Option<usize>) {
	for (i, tissue) in tissues.iter().enumerate() {
		let mut title = Span::default();
		title.content.to_mut().push(match starred {
			Some(starred) if starred == i => '*',
			_ => ' ',
		});
		title.content.to_mut().push_str(&tissue.title);
		title.content.to_mut().push(' ');
		if index == i && description_index.is_none() {
			title = title.black().on_white();
		};
		let mut title: Line = title.into();
		for tag in &tissue.tags {
			title.spans.push(format!(" ({tag})").magenta());
		}
		body.lines.push(title);
		for (di, description) in tissue.description.iter().enumerate() {
			if let Some(d_index) = description_index {
				if index == i && d_index == di {
					body.lines.push(format!(" - {description}").black().on_white().into());
					continue;
				}
			}
			body.lines.push(format!(" - {description}").dark_gray().into());
		}
	}
}

fn instructions(mode: &Mode) -> Title<'_> {
	match mode {
		Mode::Normal => Title::from(Line::from(Vec::from([
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
		]))),
		Mode::Help => Title::from(Line::from(Vec::from([" Help! ".blue().bold()]))),
		Mode::Add(title) => Title::from(Line::from(Vec::from([" Add tissue: ".blue().bold(), title.into(), "_ ".into()]))),
		Mode::Edit(title) => Title::from(Line::from(Vec::from([" Edit tissue title: ".blue().bold(), title.into(), "_ ".into()]))),
		Mode::Describe(description) => Title::from(Line::from(Vec::from([" Describe tissue: ".blue().bold(), description.into(), "_ ".into()]))),
		Mode::Tag(tag) => Title::from(Line::from(Vec::from([" Tag tissue: ".blue().bold(), tag.into(), "_ ".into()]))),
		Mode::Copy => Title::from(Line::from(Vec::from([
			" Copy what?:".blue().bold(),
			" t".red().bold(),
			"itle".into(),
			" d".red().bold(),
			"escription".into(),
			" l".red().bold(),
			"ist ".into(),
		]))),
		Mode::Publish => Title::from(Line::from(Vec::from([" Really Publish?:".blue().bold(), " y".red().bold(), "es".into(), " N".red().bold(), "o ".into()]))),
		Mode::Commit => Title::from(Line::from(Vec::from([" Really Commit?:".blue().bold(), " y".red().bold(), "es".into(), " N".red().bold(), "o ".into()]))),
		Mode::Remove => Title::from(Line::from(Vec::from([
			" Remove what?:".blue().bold(),
			" T".red().bold(),
			"issue".into(),
			" d".red().bold(),
			"escription".into(),
			" t".red().bold(),
			"ag ".into(),
		]))),
		Mode::RemoveDescription(_) => Title::from(Line::from(Vec::from([" Remove which description? ".blue().bold()]))),
		Mode::RemoveTag(tag) => Title::from(Line::from(Vec::from([" Remove tag: ".blue().bold(), tag.into(), "_ ".into()]))),
		Mode::Restore(_) => Title::from(Line::from(Vec::from([" Select tissue and restore ".blue().bold()]))),
	}
}

fn help(body: &mut Text) {
	let help = [
		Line::from("Welcome to tissuebox!".blue()),
		"".blue().into(),
		" a (add): Create a new tissue under the given name".into(),
		" d (describe): Append a description to the selected tissue".into(),
		" t (tag): Assign a tag to the selected tissue".into(),
		" e (edit): Edit the title of the selected tissue".into(),
		" r (remove): Delete the selected tissue".into(),
		// The below should be moved to an "advanced" section should they reach ~3 or 4 buttons
		" R (restore): Restore a deleted tissue".into(),
		" * (star): Marks the tissue with a *.".into(),
		"           Pressing * on a starred tissue removes the star,".into(),
		"           and pressing * from any other tissue moves the cursor to the starred issue.".into(),
		"           Useful when working on a specific tissue.".into(),
		"".into(),
		"Output commands".red().into(),
		" c (copy): Copy the title or description of the selected tissue to the clipboard".into(),
		" C (commit): Add all files to the git index and commit.".into(),
		"             Uses the selected tissue's title as the message".into(),
		"             Equivalent to `git add --all && git commit -m {title}`".into(),
		" P (publish): Publish the selected issue to GitHub. Requires the `gh` command.".into(),
	];
	*body = help.into_iter().collect();
}
