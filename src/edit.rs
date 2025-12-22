use anyhow::{Context, Result};
use console::style;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::{fs, io, path::PathBuf};

use torrite::cli::EditArgs;
use torrite::models::Torrent;

pub fn edit_torrent(args: EditArgs) -> Result<()> {
    let content = fs::read(&args.torrent).context("Failed to read torrent file")?;
    let mut torrent: Torrent =
        serde_bencode::from_bytes(&content).context("Invalid torrent file")?;

    // Check if any modification flags are set (headless mode)
    let headless = !args.announce.is_empty()
        || args.replace_announce.is_some()
        || args.comment.is_some()
        || args.private
        || args.public;

    if headless {
        if apply_changes(&mut torrent, &args) {
            let output_path = args.output.unwrap_or(args.torrent);
            println!("Saving to: {}", style(output_path.display()).cyan());

            let bencode_data =
                serde_bencode::to_bytes(&torrent).context("Failed to serialize torrent")?;
            fs::write(output_path, bencode_data).context("Failed to write torrent file")?;
        } else {
            println!("No changes made.");
        }
    } else {
        // TUI mode
        let output_path = args.output.unwrap_or(args.torrent.clone());
        run_tui(torrent, output_path)?;
    }

    Ok(())
}

fn run_tui(mut torrent: Torrent, path: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal, &mut torrent, path);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = app_result {
        println!("Error: {:?}", err);
    }

    Ok(())
}

struct App {
    torrent: Torrent,
    path: PathBuf,
    list_state: ListState,
    items: Vec<&'static str>,
    editing: bool,
    input: String,
    // Dialog states
    show_save_quit_dialog: bool,
    show_unsaved_quit_dialog: bool,
    dialog_selection: bool, // true = Yes, false = No
    is_dirty: bool,
}

impl App {
    fn new(torrent: Torrent, path: PathBuf) -> App {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        App {
            torrent,
            path,
            list_state,
            items: vec!["Announce URL", "Comment", "Private"],
            editing: false,
            input: String::new(),
            show_save_quit_dialog: false,
            show_unsaved_quit_dialog: false,
            dialog_selection: true,
            is_dirty: false,
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn get_value(&self, index: usize) -> String {
        match index {
            0 => self.torrent.announce.clone().unwrap_or_default(),
            1 => self.torrent.comment.clone().unwrap_or_default(),
            2 => {
                if self.torrent.info.private == Some(1) {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            _ => String::new(),
        }
    }

    fn set_value(&mut self, index: usize, value: String) {
        let old_value = self.get_value(index);
        if old_value != value {
            self.is_dirty = true;
        }

        match index {
            0 => {
                self.torrent.announce = if value.is_empty() {
                    None
                } else {
                    Some(value.clone())
                };
                // Also update first tier of announce list if it exists, roughly
                if let Some(list) = &mut self.torrent.announce_list {
                    if !list.is_empty() && !list[0].is_empty() {
                        if let Some(ann) = &self.torrent.announce {
                            list[0][0] = ann.clone();
                        }
                    } else if list.is_empty() && !value.is_empty() {
                        // Create list if it doesn't exist
                        list.push(vec![value]);
                    }
                } else if !value.is_empty() {
                    self.torrent.announce_list = Some(vec![vec![value]]);
                }
            }
            1 => self.torrent.comment = if value.is_empty() { None } else { Some(value) },
            _ => {}
        }
    }
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    torrent: &mut Torrent,
    path: PathBuf,
) -> Result<()> {
    let mut app = App::new(torrent.clone(), path);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if app.show_save_quit_dialog || app.show_unsaved_quit_dialog {
                    match key.code {
                        KeyCode::Left | KeyCode::Right => {
                            app.dialog_selection = !app.dialog_selection;
                        }
                        KeyCode::Enter => {
                            if app.show_save_quit_dialog {
                                if app.dialog_selection {
                                    // Yes -> Quit
                                    return Ok(());
                                } else {
                                    // No -> Close dialog
                                    app.show_save_quit_dialog = false;
                                }
                            } else if app.show_unsaved_quit_dialog {
                                if app.dialog_selection {
                                    // Yes -> Quit
                                    return Ok(());
                                } else {
                                    // No -> Close dialog
                                    app.show_unsaved_quit_dialog = false;
                                }
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.show_save_quit_dialog = false;
                            app.show_unsaved_quit_dialog = false;
                        }
                        _ => {}
                    }
                } else if app.editing {
                    match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app.list_state.selected() {
                                app.set_value(idx, app.input.clone());
                            }
                            app.editing = false;
                        }
                        KeyCode::Esc => {
                            app.editing = false;
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => {
                            if app.is_dirty {
                                app.show_unsaved_quit_dialog = true;
                                app.dialog_selection = false; // Default to No
                            } else {
                                return Ok(());
                            }
                        }
                        KeyCode::Char('s') => {
                            let bencode_data = serde_bencode::to_bytes(&app.torrent)
                                .context("Failed to serialize torrent")?;
                            fs::write(&app.path, bencode_data)
                                .context("Failed to write torrent file")?;
                            app.is_dirty = false;
                            app.show_save_quit_dialog = true;
                            app.dialog_selection = true; // Default to Yes
                        }
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::Enter => {
                            if let Some(idx) = app.list_state.selected() {
                                match idx {
                                    0 | 1 => {
                                        app.editing = true;
                                        app.input = app.get_value(idx);
                                    }
                                    2 => {
                                        // Toggle Private
                                        let old_val = app.torrent.info.private;
                                        if app.torrent.info.private == Some(1) {
                                            app.torrent.info.private = None;
                                        } else {
                                            app.torrent.info.private = Some(1);
                                        }
                                        if old_val != app.torrent.info.private {
                                            app.is_dirty = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title_text = format!("Editing: {}", app.path.display());
    let title = Paragraph::new(title_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Torrite Editor"),
    );
    f.render_widget(title, chunks[0]);

    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = format!("{}: {}", m, app.get_value(i));
            let style = Style::default().add_modifier(Modifier::BOLD);
            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Fields"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    if app.editing {
        let input = Paragraph::new(app.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Edit Value"));
        f.render_widget(input, chunks[2]);
    } else {
        let help_text = if app.is_dirty {
            "Use Arrow Keys to navigate, Enter to edit, s to save, q to quit (Unsaved Changes!)"
        } else {
            "Use Arrow Keys to navigate, Enter to edit, s to save, q to quit"
        };
        let help = Paragraph::new(help_text).style(Style::default().fg(if app.is_dirty {
            Color::Red
        } else {
            Color::Gray
        }));
        f.render_widget(help, chunks[2]);
    }

    if app.show_save_quit_dialog || app.show_unsaved_quit_dialog {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area); // Clear background

        let title = if app.show_save_quit_dialog {
            "File Saved"
        } else {
            "Unsaved Changes"
        };

        let block = Block::default().title(title).borders(Borders::ALL);
        f.render_widget(block, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Text
                Constraint::Length(1), // Buttons
            ])
            .margin(1)
            .split(area);

        let text_str = if app.show_save_quit_dialog {
            "File saved successfully. Do you want to quit?"
        } else {
            "You have unsaved changes. Do you want to quit anyway?"
        };

        let text = Paragraph::new(text_str)
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(text, layout[0]);

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(7), // 3 for "Yes" + 2 padding each side
                Constraint::Length(2),
                Constraint::Length(6), // 2 for "No" + 2 padding each side
                Constraint::Min(0),
            ])
            .split(layout[1]);

        let yes_style = if app.dialog_selection {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::Green)
        };

        let no_style = if !app.dialog_selection {
            Style::default().fg(Color::Black).bg(Color::Red)
        } else {
            Style::default().fg(Color::Red)
        };

        let yes_btn = Paragraph::new("Yes")
            .style(yes_style)
            .alignment(ratatui::layout::Alignment::Center);

        let no_btn = Paragraph::new("No")
            .style(no_style)
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(yes_btn, button_layout[1]);
        f.render_widget(no_btn, button_layout[3]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn apply_changes(torrent: &mut Torrent, args: &EditArgs) -> bool {
    let mut modified = false;

    // Announce
    if let Some(ref new_announce) = args.replace_announce {
        println!("Replaced announce with: {}", new_announce);
        torrent.announce = Some(new_announce.clone());
        torrent.announce_list = Some(vec![vec![new_announce.clone()]]);
        modified = true;
    } else if !args.announce.is_empty() {
        let mut list = torrent.announce_list.clone().unwrap_or_else(Vec::new);
        // Append as new tiers
        for url in &args.announce {
            println!("Added announce: {}", url);
            list.push(vec![url.clone()]);
        }
        // If main announce was empty, set it to the first one
        if torrent.announce.is_none() && !list.is_empty() {
            torrent.announce = Some(list[0][0].clone());
        }
        torrent.announce_list = Some(list);
        modified = true;
    }

    // Comment
    if let Some(ref comment) = args.comment {
        println!("Updated comment: {}", comment);
        torrent.comment = Some(comment.clone());
        modified = true;
    }

    // Private
    if args.private {
        if torrent.info.private != Some(1) {
            println!("Set private flag.");
            torrent.info.private = Some(1);
            modified = true;
        }
    } else if args.public {
        if torrent.info.private.is_some() {
            println!("Removed private flag.");
            torrent.info.private = None;
            modified = true;
        }
    }

    modified
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use torrite::models::Info;

    fn create_dummy_torrent() -> Torrent {
        Torrent {
            announce: None,
            announce_list: None,
            comment: None,
            created_by: "test".to_string(),
            creation_date: None,
            info: Info {
                piece_length: 1024,
                pieces: None,
                name: "test".to_string(),
                private: None,
                files: None,
                length: Some(100),
                source: None,
                x_cross_seed: None,
                meta_version: None,
                file_tree: None,
            },
            url_list: None,
            piece_layers: None,
        }
    }

    #[test]
    fn test_apply_changes_comment() {
        let mut torrent = create_dummy_torrent();
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: None,
            comment: Some("New Comment".to_string()),
            private: false,
            public: false,
            output: None,
        };

        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.comment.unwrap(), "New Comment");
    }

    #[test]
    fn test_apply_changes_announce_replace() {
        let mut torrent = create_dummy_torrent();
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: Some("http://new.tracker".to_string()),
            comment: None,
            private: false,
            public: false,
            output: None,
        };

        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.announce.unwrap(), "http://new.tracker");
        assert_eq!(torrent.announce_list.unwrap().len(), 1);
    }

    #[test]
    fn test_apply_changes_private() {
        let mut torrent = create_dummy_torrent();
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: None,
            comment: None,
            private: true,
            public: false,
            output: None,
        };

        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.info.private, Some(1));

        // No change if already private
        assert!(!apply_changes(&mut torrent, &args));
    }

    #[test]
    fn test_apply_changes_public() {
        let mut torrent = create_dummy_torrent();
        torrent.info.private = Some(1);
        let args = EditArgs {
            torrent: PathBuf::from("test.torrent"),
            announce: vec![],
            replace_announce: None,
            comment: None,
            private: false,
            public: true,
            output: None,
        };

        assert!(apply_changes(&mut torrent, &args));
        assert_eq!(torrent.info.private, None);
    }
}
