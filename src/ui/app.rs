use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::user::User;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::DynamicImage;
use ratatui::symbols;
use ratatui::widgets::Tabs;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph, TableState, Wrap},
};

#[derive(Default)]
pub struct App {
    pub(super) running: bool,
    pub(super) user: User,
    pub(super) favorites: Arc<Mutex<Vec<Option<DynamicImage>>>>,
    pub(super) selected_tab: usize,
    pub(super) diary_table_state: TableState,
    pub(super) watchlist_table_state: TableState,
    pub(super) poster_cache: Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
    pub(super) pending_fetches: Arc<Mutex<HashSet<(String, i32)>>>,
    pub(super) diary_prefetch_cursor: usize,
    pub(super) watchlist_prefetch_cursor: usize,
    pub(super) visualizer_input: String,
    pub(super) visualizer_query: String,
}

impl App {
    pub fn new(user: User, favorites: Arc<Mutex<Vec<Option<DynamicImage>>>>) -> Self {
        let mut diary_table_state = TableState::default();
        diary_table_state.select_first();
        diary_table_state.select_first_column();

        let mut watchlist_table_state = TableState::default();
        watchlist_table_state.select_first();
        watchlist_table_state.select_first_column();

        Self {
            running: true,
            user,
            favorites,
            selected_tab: 0,
            diary_table_state,
            watchlist_table_state,
            poster_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_fetches: Arc::new(Mutex::new(HashSet::new())),
            diary_prefetch_cursor: 0,
            watchlist_prefetch_cursor: 0,
            visualizer_input: String::new(),
            visualizer_query: String::new(),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let buffer = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Min(0)])
            .split(frame.area());

        Self::render_title_bar(frame, buffer[0], self.selected_tab);
        self.render_content(frame, buffer[1], self.selected_tab);
    }

    fn render_title_bar(frame: &mut Frame, area: Rect, selected_tab: usize) {
        let title = Line::from("termboxd").bold().white().centered();
        let block = Block::bordered().title(title).style(Color::White);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let halves = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        Self::render_tabs(frame, halves[0], selected_tab);

        let message_area = Rect {
            width: halves[1].width.saturating_sub(2),
            ..halves[1]
        };
        let message = Paragraph::new("Press `Ctrl-C` or `q` to exit")
            .right_aligned()
            .style(Color::White);
        frame.render_widget(message, message_area);
    }

    pub fn render_tabs(frame: &mut Frame, area: Rect, selected_tab: usize) {
        let highlight_color = match selected_tab {
            0 => Color::White,
            1 => Color::Rgb(255, 128, 0),
            2 => Color::Rgb(0, 224, 84),
            3 => Color::Rgb(64, 188, 244),
            _ => unreachable!(),
        };
        let tabs = Tabs::new(vec!["[P]rofile", "[D]iary", "[W]atchlist", "[V]isualizer"])
            .style(Color::White)
            .highlight_style(Style::default().fg(highlight_color).bold())
            .select(selected_tab)
            .divider(symbols::DOT)
            .padding(" ", " ");
        frame.render_widget(tabs, area);
    }

    pub fn render_content(&mut self, frame: &mut Frame, area: Rect, selected_tab: usize) {
        match selected_tab {
            0 => self.render_homepage(frame, area),
            1 => self.render_diary(frame, area),
            2 => self.render_watchlist(frame, area),
            3 => self.render_visualizer(frame, area),
            _ => unreachable!(),
        }
    }

    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        let timeout = Duration::from_millis(100);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        if self.selected_tab == 3 {
            self.on_visualizer_key_event(key);
            return;
        }
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Char('P') | KeyCode::Char('p')) => self.selected_tab = 0,
            (_, KeyCode::Char('D') | KeyCode::Char('d')) => self.selected_tab = 1,
            (_, KeyCode::Char('W') | KeyCode::Char('w')) => self.selected_tab = 2,
            (_, KeyCode::Char('V') | KeyCode::Char('v')) => self.selected_tab = 3,
            (_, KeyCode::Char('j') | KeyCode::Down) => match self.selected_tab {
                1 => self.diary_table_state.select_next(),
                2 => self.watchlist_table_state.select_next(),
                _ => {}
            },
            (_, KeyCode::Char('k') | KeyCode::Up) => match self.selected_tab {
                1 => self.diary_table_state.select_previous(),
                2 => self.watchlist_table_state.select_previous(),
                _ => {}
            },
            _ => {}
        }
    }

    pub(super) fn quit(&mut self) {
        self.running = false;
    }

    pub(super) fn render_text_block(
        frame: &mut Frame,
        area: Rect,
        title: &str,
        color: Color,
        header: &str,
        body: &str,
    ) {
        let block = Block::bordered()
            .title(title)
            .border_style(Style::default().fg(color));
        let inner = block.inner(area).inner(Margin::new(1, 1));
        frame.render_widget(block, area);

        let header_lines = Paragraph::new(header.to_string())
            .wrap(Wrap { trim: true })
            .line_count(inner.width) as u16;
        let body_budget = inner.height.saturating_sub(header_lines);
        let body = Self::fit_text(body, inner.width, body_budget);

        let text = if body.is_empty() {
            header.to_string()
        } else {
            format!("{header}\n{body}")
        };
        let paragraph = Paragraph::new(text).centered().wrap(Wrap { trim: true });
        let line_count = paragraph.line_count(inner.width) as u16;
        let text_rect = inner.centered_vertically(Constraint::Length(line_count));
        frame.render_widget(paragraph, text_rect);
    }

    pub(super) fn fit_text(text: &str, width: u16, max_lines: u16) -> String {
        if max_lines == 0 || text.is_empty() {
            return String::new();
        }
        let fits = |s: &str| {
            Paragraph::new(s.to_string())
                .wrap(Wrap { trim: true })
                .line_count(width) as u16
                <= max_lines
        };
        if fits(text) {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut lo = 0;
        let mut hi = chars.len();
        while lo < hi {
            let mid = lo + (hi - lo + 1) / 2;
            let candidate = format!("{}...", chars[..mid].iter().collect::<String>());
            if fits(&candidate) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        format!("{}...", chars[..lo].iter().collect::<String>())
    }
}
