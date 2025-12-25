use anyhow::Result;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io;
use std::path::{MAIN_SEPARATOR, PathBuf};
use std::time::Duration;

use torrite::cli::CreateArgs;
use torrite::config::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    InputSelection,
    Metadata,
    OutputSelection,
    Summary,
}

struct App {
    step: Step,
    config: Config,

    // Data being built
    source: Option<PathBuf>,
    profile_idx: usize,
    available_profiles: Vec<String>,

    // Metadata fields
    announce: String, // Comma separated or newlines
    comment: String,
    piece_length: String, // Input string, parsed later
    private: bool,
    source_string: String,
    web_seeds: String,

    // Output
    output_path: String,

    // UI State
    metadata_list_state: ListState,
    metadata_editing_idx: Option<usize>, // If Some, we are typing in a field
    input_buffer: String,                // Buffer for current editing

    // Dialog states
    show_quit_dialog: bool,
    dialog_selection: bool, // true = Yes, false = No
    is_dirty: bool,
}

impl App {
    fn new(config: Config) -> Self {
        let mut profiles: Vec<String> = config.profiles.keys().cloned().collect();
        profiles.sort();
        profiles.insert(0, "None".to_string());

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            step: Step::InputSelection,
            config,
            source: None,
            profile_idx: 0,
            available_profiles: profiles,
            announce: String::new(),
            comment: String::new(),
            piece_length: String::new(),
            private: false,
            source_string: String::new(),
            web_seeds: String::new(),
            output_path: String::new(),
            metadata_list_state: list_state,
            metadata_editing_idx: None,
            input_buffer: String::new(),
            show_quit_dialog: false,
            dialog_selection: false,
            is_dirty: false,
        }
    }

    fn apply_profile(&mut self) {
        if self.profile_idx == 0 {
            return; // None selected
        }
        let name = &self.available_profiles[self.profile_idx];
        if let Some(profile) = self.config.profiles.get(name) {
            if let Some(ann) = &profile.announce {
                self.announce = ann.join("\n");
            }
            if let Some(comm) = &profile.comment {
                self.comment = comm.clone();
            }
            if let Some(priv_flag) = profile.private {
                self.private = priv_flag;
            }
            if let Some(pl) = profile.piece_length {
                self.piece_length = pl.to_string();
            }
            if let Some(src) = &profile.source_string {
                self.source_string = src.clone();
            }
            if let Some(ws) = &profile.web_seed {
                self.web_seeds = ws.join("\n");
            }
        }
    }

    fn to_args(&self) -> CreateArgs {
        let announce_vec: Vec<String> = self
            .announce
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let web_seed_vec: Vec<String> = self
            .web_seeds
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        CreateArgs {
            source: self.source.clone(),
            profile: if self.profile_idx > 0 {
                Some(self.available_profiles[self.profile_idx].clone())
            } else {
                None
            },
            announce: announce_vec,
            comment: if self.comment.is_empty() {
                None
            } else {
                Some(self.comment.clone())
            },
            no_date: false,  // Not exposed in UI for simplicity
            exclude: vec![], // Not exposed
            force: false,    // Will be handled by main logic possibly, or we assume force
            piece_length: self.piece_length.parse().ok(),
            name: None, // Auto-derive
            output: if self.output_path.is_empty() {
                None
            } else {
                Some(PathBuf::from(&self.output_path))
            },
            date: None,
            private: self.private,
            source_string: if self.source_string.is_empty() {
                None
            } else {
                Some(self.source_string.clone())
            },
            threads: None,
            verbose: false,
            web_seed: web_seed_vec,
            cross_seed: false,
            info_hash: false,
            json: false,
            v2: false, // Default to v1/hybrid depending on detection, or add toggle
            hybrid: false,
            dry_run: false,
        }
    }
}

pub fn run(config: Config) -> Result<Option<CreateArgs>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal, App::new(config));

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    app_result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<Option<CreateArgs>> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // 1. Wait for the first event (with timeout for redraws)
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        // 2. "Slurp" all pending events into a batch
        // This captures drag-and-drop on Windows, which injects keys in rapid bursts
        let mut events = vec![event::read()?];
        // Use 50ms timeout to catch events that arrive in quick succession
        // (drag-and-drop sends events very fast but not always instantaneously)
        while event::poll(Duration::from_millis(50))? {
            events.push(event::read()?);
        }

        // 3. Process the batch - separate characters from special keys
        let mut chars_received = String::new();
        let mut special_keys = Vec::new();
        let mut paste_events = Vec::new();

        for e in events {
            match e {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char(c) => chars_received.push(c),
                    _ => special_keys.push(key),
                },
                Event::Paste(s) => paste_events.push(s),
                _ => {} // Ignore mouse/resize in batch
            }
        }

        // 4. Handle batched character input (likely drag-and-drop or paste)
        if !chars_received.is_empty() || !paste_events.is_empty() {
            // Combine all pasted/typed content
            let mut combined = chars_received;
            for paste in paste_events {
                combined.push_str(&paste);
            }

            // Clean up Windows drag-and-drop artifacts
            let clean_data = combined
                .trim()
                .replace("\"", "")
                .replace("'", "")
                .replace("file://", "");

            // Expand shell aliases like ~ to home directory
            let expanded_path = shellexpand::tilde(&clean_data).to_string();

            // Detect if this looks like a file path
            let looks_like_path = expanded_path.contains(MAIN_SEPARATOR)
                || expanded_path.contains('/')
                || clean_data.starts_with('~');
            let is_not_root = expanded_path != "/" && expanded_path != "\\";

            // Validate based on current step
            let is_valid_path = if app.metadata_editing_idx.is_none() {
                match app.step {
                    Step::InputSelection => {
                        // Input must exist and not be root
                        looks_like_path
                            && std::path::Path::new(&expanded_path).exists()
                            && is_not_root
                    }
                    Step::OutputSelection => {
                        // Output just needs to look like a path and not be root
                        looks_like_path && is_not_root
                    }
                    _ => false,
                }
            } else {
                false
            };

            if is_valid_path {
                // Handle as file path based on current step
                match app.step {
                    Step::InputSelection => {
                        app.source = Some(PathBuf::from(expanded_path));
                        app.is_dirty = true;
                    }
                    Step::OutputSelection => {
                        let path = PathBuf::from(&expanded_path);
                        if path.is_dir() {
                            if let Some(src) = &app.source {
                                let name = src.file_name().unwrap_or_default();
                                app.output_path = path
                                    .join(name)
                                    .with_extension("torrent")
                                    .to_string_lossy()
                                    .to_string();
                            } else {
                                app.output_path =
                                    path.join("output.torrent").to_string_lossy().to_string();
                            }
                        } else {
                            app.output_path = expanded_path;
                        }
                        app.is_dirty = true;
                    }
                    _ => {} // Ignore in other steps
                }
                // Skip processing special keys - the path was the main event
                continue;
            } else if app.metadata_editing_idx.is_some() {
                // If actively editing, append the characters to input buffer
                app.input_buffer.push_str(&combined);
                // Don't process special keys if we just handled text input
                if special_keys.is_empty() {
                    continue;
                }
            }
        }

        // 5. Process special keys (navigation, Enter, Esc, etc.)
        for key in special_keys {
            // Handle quit dialog if active
            if app.show_quit_dialog {
                match key.code {
                    KeyCode::Left | KeyCode::Right => {
                        app.dialog_selection = !app.dialog_selection;
                    }
                    KeyCode::Enter => {
                        if app.dialog_selection {
                            // Yes -> Quit without saving
                            return Ok(None);
                        } else {
                            // No -> Close dialog
                            app.show_quit_dialog = false;
                        }
                    }
                    KeyCode::Esc => {
                        // Cancel dialog
                        app.show_quit_dialog = false;
                    }
                    _ => {}
                }
                continue;
            }

            // Global quit (Esc)
            if app.metadata_editing_idx.is_none() && key.code == KeyCode::Esc {
                if app.is_dirty {
                    app.show_quit_dialog = true;
                    app.dialog_selection = false; // Default to No
                } else {
                    return Ok(None);
                }
                continue;
            }

            // If editing text
            if let Some(idx) = app.metadata_editing_idx {
                match key.code {
                    KeyCode::Enter => {
                        // Commit change
                        match idx {
                            0 => {} // Profile - handled differently
                            1 => {
                                app.comment = app.input_buffer.clone();
                                app.is_dirty = true;
                            }
                            2 => {
                                app.piece_length = app.input_buffer.clone();
                                app.is_dirty = true;
                            }
                            3 => {} // Private - checkbox
                            4 => {
                                app.source_string = app.input_buffer.clone();
                                app.is_dirty = true;
                            }
                            5 => {
                                app.web_seeds = app.input_buffer.clone();
                                app.is_dirty = true;
                            }
                            6 => {
                                app.announce = app.input_buffer.clone();
                                app.is_dirty = true;
                            }
                            999 => {
                                app.output_path = app.input_buffer.clone();
                                app.is_dirty = true;
                            }
                            _ => {}
                        }
                        app.metadata_editing_idx = None;
                    }
                    KeyCode::Esc => {
                        // Cancel edit
                        app.metadata_editing_idx = None;
                    }
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        // Individual character input while editing
                        // (Batched input was already handled above)
                        app.input_buffer.push(c);
                    }
                    _ => {} // Ignore other keys while editing
                }
                continue;
            }

            // Handle navigation and actions based on current step
            match app.step {
                Step::InputSelection => match key.code {
                    KeyCode::Tab | KeyCode::Enter => {
                        if app.source.is_some() {
                            app.step = Step::Metadata;
                        }
                    }
                    _ => {} // Ignore other keys
                },
                Step::Metadata => match key.code {
                    KeyCode::Tab => {
                        app.step = Step::OutputSelection;
                        // Auto-suggest output path based on source
                        if app.output_path.is_empty() {
                            if let Some(src) = &app.source {
                                let file_name =
                                    src.file_name().unwrap_or_default().to_string_lossy();
                                app.output_path = format!("{}.torrent", file_name);
                            }
                        }
                    }
                    KeyCode::BackTab => {
                        app.step = Step::InputSelection;
                    }
                    KeyCode::Down => {
                        let i = match app.metadata_list_state.selected() {
                            Some(i) => {
                                if i >= 6 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        app.metadata_list_state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = match app.metadata_list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    6
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        app.metadata_list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = app.metadata_list_state.selected() {
                            match idx {
                                0 => {
                                    // Profile
                                    if app.profile_idx + 1 < app.available_profiles.len() {
                                        app.profile_idx += 1;
                                    } else {
                                        app.profile_idx = 0;
                                    }
                                    app.apply_profile();
                                    app.is_dirty = true;
                                }
                                3 => {
                                    // Private
                                    app.private = !app.private;
                                    app.is_dirty = true;
                                }
                                _ => {
                                    app.metadata_editing_idx = Some(idx);
                                    app.input_buffer = match idx {
                                        1 => app.comment.clone(),
                                        2 => app.piece_length.clone(),
                                        4 => app.source_string.clone(),
                                        5 => app.web_seeds.clone(),
                                        6 => app.announce.clone(),
                                        _ => String::new(),
                                    };
                                }
                            }
                        }
                    }
                    _ => {} // Ignore other keys
                },
                Step::OutputSelection => match key.code {
                    KeyCode::Tab => {
                        app.step = Step::Summary;
                    }
                    KeyCode::BackTab => {
                        app.step = Step::Metadata;
                    }
                    KeyCode::Enter => {
                        // Edit path manually
                        app.metadata_editing_idx = Some(999); // Special ID for output path
                        app.input_buffer = app.output_path.clone();
                    }
                    _ => {} // Ignore other keys
                },
                Step::Summary => match key.code {
                    KeyCode::Enter | KeyCode::Char('y') => {
                        return Ok(Some(app.to_args()));
                    }
                    KeyCode::BackTab => {
                        app.step = Step::OutputSelection;
                    }
                    _ => {} // Ignore other keys
                },
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let main_block = Block::default()
        .borders(Borders::ALL)
        .title(" Torrite Creator ");
    f.render_widget(main_block, f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Progress/Steps
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Help bar
        ])
        .split(f.area());

    // Render Step Indicator
    let steps_text = vec![
        if app.step == Step::InputSelection {
            "1. Input".black().on_white()
        } else {
            "1. Input".into()
        },
        " -> ".into(),
        if app.step == Step::Metadata {
            "2. Metadata".black().on_white()
        } else {
            "2. Metadata".into()
        },
        " -> ".into(),
        if app.step == Step::OutputSelection {
            "3. Output".black().on_white()
        } else {
            "3. Output".into()
        },
        " -> ".into(),
        if app.step == Step::Summary {
            "4. Summary".black().on_white()
        } else {
            "4. Summary".into()
        },
    ];
    let steps = Paragraph::new(Line::from(steps_text))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(steps, chunks[0]);

    // Main Content
    let content_area = chunks[1];

    match app.step {
        Step::InputSelection => {
            let text = if let Some(path) = &app.source {
                vec![
                    Line::from(vec![Span::raw("Selected Input: ")]),
                    Line::from(vec![Span::styled(
                        path.display().to_string(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from("Press Tab or Enter to continue, or drag & drop another file to replace."),
                ]
            } else {
                vec![
                    Line::from("Drag and drop a file or directory here to begin."),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "(Waiting for input...)",
                        Style::default().dim(),
                    )]),
                ]
            };

            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::NONE)); // Inner content

            // Vertically center
            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(4),
                    Constraint::Min(0),
                ])
                .split(content_area);

            f.render_widget(p, v_chunks[1]);
        }
        Step::Metadata => {
            let items = vec![
                format!("Profile:      {}", app.available_profiles[app.profile_idx]),
                format!("Comment:      {}", app.comment),
                format!(
                    "Piece Len:    {}",
                    if app.piece_length.is_empty() {
                        "Auto"
                    } else {
                        &app.piece_length
                    }
                ),
                format!("Private:      {}", if app.private { "Yes" } else { "No" }),
                format!("Source:       {}", app.source_string),
                format!("Web Seeds:    {}", app.web_seeds.replace('\n', ", ")),
                format!("Announce URLs: {}", app.announce.lines().count()),
            ];

            let list_items: Vec<ListItem> =
                items.iter().map(|i| ListItem::new(i.as_str())).collect();

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(" Metadata "))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            // Layout for metadata and announce box
            let meta_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(content_area);

            f.render_stateful_widget(list, meta_chunks[0], &mut app.metadata_list_state);

            // Announce preview box
            let announce_text = app.announce.clone();
            let p = Paragraph::new(announce_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Trackers (Preview) "),
            );
            f.render_widget(p, meta_chunks[1]);

            // Editing popup
            if let Some(idx) = app.metadata_editing_idx {
                if idx != 999 {
                    // Not output editing
                    let area = centered_rect(60, 20, f.area());
                    f.render_widget(Clear, area);
                    let title = match idx {
                        1 => "Edit Comment",
                        2 => "Edit Piece Length (e.g. 18 for 256KB)",
                        4 => "Edit Source String",
                        5 => "Edit Web Seeds (newline separated)",
                        6 => "Edit Announce URLs (newline separated)",
                        _ => "Edit",
                    };
                    let input = Paragraph::new(app.input_buffer.as_str())
                        .block(Block::default().borders(Borders::ALL).title(title))
                        .style(Style::default().fg(Color::Yellow));
                    f.render_widget(input, area);
                }
            }
        }
        Step::OutputSelection => {
            let text = vec![
                Line::from("Drag and drop a folder or file to set the output path."),
                Line::from(""),
                Line::from(vec![Span::raw("Current Output Path: ")]),
                Line::from(vec![Span::styled(
                    if app.output_path.is_empty() {
                        "None"
                    } else {
                        &app.output_path
                    },
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(""),
                Line::from("Press Enter to edit, Tab to continue, Shift+Tab to go back."),
            ];
            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(6),
                    Constraint::Min(0),
                ])
                .split(content_area);
            f.render_widget(p, v_chunks[1]);

            if let Some(999) = app.metadata_editing_idx {
                let area = centered_rect(60, 10, f.area());
                f.render_widget(Clear, area);
                let input = Paragraph::new(app.input_buffer.as_str())
                    .block(Block::default().borders(Borders::ALL).title("Output Path"))
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(input, area);
            }
        }
        Step::Summary => {
            let summary_text = vec![
                Line::from(vec![Span::styled(
                    "Review Settings",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(format!(
                    "Input:   {:?}",
                    app.source.as_ref().unwrap_or(&PathBuf::from("None"))
                )),
                Line::from(format!("Output:  {}", app.output_path)),
                Line::from(format!("Comment: {}", app.comment)),
                Line::from(format!("Private: {}", app.private)),
                Line::from(format!(
                    "Trackers: {} defined",
                    app.announce.lines().count()
                )),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Press Enter to Create Torrent",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from("Press Shift+Tab to go back and make changes."),
            ];

            let p = Paragraph::new(summary_text)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(10),
                    Constraint::Percentage(80),
                    Constraint::Min(0),
                ])
                .split(content_area);

            f.render_widget(p, v_chunks[1]);
        }
    }

    // Render quit confirmation dialog if active
    if app.show_quit_dialog {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area); // Clear background

        let block = Block::default()
            .title("Unsaved Changes")
            .borders(Borders::ALL);
        f.render_widget(block, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Text
                Constraint::Length(1), // Buttons
            ])
            .margin(1)
            .split(area);

        let text = Paragraph::new("You have unsaved changes. Do you want to quit anyway?")
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
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
            .alignment(Alignment::Center);

        let no_btn = Paragraph::new("No")
            .style(no_style)
            .alignment(Alignment::Center);

        f.render_widget(yes_btn, button_layout[1]);
        f.render_widget(no_btn, button_layout[3]);
    }

    // Render help bar at bottom
    let help_text = match app.step {
        Step::InputSelection => {
            if app.source.is_some() {
                "Esc: Quit | Tab/Enter: Continue"
            } else {
                "Esc: Quit | Drag & drop a file or directory to begin"
            }
        }
        Step::Metadata => "Esc: Quit | Tab: Continue | Shift+Tab: Back | ↑/↓: Navigate | Enter: Edit/Toggle",
        Step::OutputSelection => "Esc: Quit | Tab: Continue | Shift+Tab: Back | Enter: Edit path",
        Step::Summary => "Esc: Quit | Enter: Create | Shift+Tab: Back",
    };

    let help_bar = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(help_bar, chunks[2]);
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
