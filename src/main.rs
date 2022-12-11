use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};

const DB_PATH: &str = "./out/db.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Clone)]
struct Contributor {
    id: usize,
    name: String,
    email: String,
    username: String,
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Contributors,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Contributors => 1,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(250);

    thread::spawn(move || {
        let mut last_tick = Instant::now();

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("read works") {
                    tx.send(Event::Input(key)).expect("send works");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_labels = vec!["Home", "Contributors", "Quit"];
    let mut active_menu_item = MenuItem::Home;
    let mut contributors_list_state = ListState::default();
    contributors_list_state.select(Some(0));

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let copyright = Paragraph::new("Base TUI 2022 @mateonunez - All rights reserved")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().bg(Color::Black))
                        .title("Copyright")
                        .border_type(BorderType::Plain),
                );

            let menu = menu_labels
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            match active_menu_item {
                MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Contributors => {
                    let contributors_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);

                    let (left, right) = render_contributors(&contributors_list_state);
                    rect.render_stateful_widget(
                        left,
                        contributors_chunks[0],
                        &mut contributors_list_state,
                    );
                    rect.render_widget(right, contributors_chunks[1]);
                }
            }
            rect.render_widget(copyright, chunks[2]);
        })?;

        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('h') => active_menu_item = MenuItem::Home,
                KeyCode::Char('c') => active_menu_item = MenuItem::Contributors,
                KeyCode::Up => {
                    if let Some(selected) = contributors_list_state.selected() {
                        let amount_contributors =
                            read_db().expect("can fetch contributor list").len();

                        if selected > 0 {
                            contributors_list_state.select(Some(selected - 1));
                        } else {
                            contributors_list_state.select(Some(amount_contributors - 1));
                        }
                    }
                }
                KeyCode::Down => {
                    if let Some(selected) = contributors_list_state.selected() {
                        let amount_contributors =
                            read_db().expect("can fetch contributor list").len();

                        if selected < amount_contributors - 1 {
                            contributors_list_state.select(Some(selected + 1));
                        } else {
                            contributors_list_state.select(Some(0));
                        }
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
    }

    Ok(())
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "Base TUI",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'c' to access contributors.")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );
    home
}

fn render_contributors<'a>(state: &ListState) -> (List<'a>, Table<'a>) {
    let contributors = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("contributors")
        .border_type(BorderType::Plain);

    let contributor_list = read_db().expect("can fetch contributor list");
    let items: Vec<_> = contributor_list
        .iter()
        .map(|contributor| {
            ListItem::new(Spans::from(vec![Span::styled(
                contributor.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_contributor = contributor_list
        .get(
            state
                .selected()
                .expect("there is always a selected contributor"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(contributors).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let contributor_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_contributor.id.to_string())),
        Cell::from(Span::raw(selected_contributor.name)),
        Cell::from(Span::raw(selected_contributor.email)),
        Cell::from(Span::raw(selected_contributor.username)),
    ])])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            "ID",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Name",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Email",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Username",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Info")
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(5),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(30),
        Constraint::Percentage(20),
    ]);

    (list, contributor_detail)
}

fn read_db() -> Result<Vec<Contributor>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<Contributor> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}
