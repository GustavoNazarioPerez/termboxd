use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;

use crate::log;
use crate::movies::{Movie, PosterWidget, Stats, strip_html_tags};
use crate::user::User;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::DynamicImage;
use ratatui::symbols;
use ratatui::widgets::{Row, Table, TableState, Tabs};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Bar, BarChart, Block, Borders, Paragraph, Wrap},
};

const LABELS: [&str; 11] = [
    "0", ".5", "1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5", "5",
];
const BAR_WIDTH: u16 = 4;

#[derive(Default)]
pub struct App {
    running: bool,
    user: User,
    favorites: Vec<DynamicImage>,
    stats: Stats,
    selected_tab: usize,
    diary_table_state: TableState,
    watchlist_table_state: TableState,
    poster_cache: Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
    pending_fetches: Arc<Mutex<HashSet<(String, i32)>>>,
    diary_prefetch_cursor: usize,
    watchlist_prefetch_cursor: usize,
}

impl App {
    pub fn new(user: User, favorites: Vec<DynamicImage>, stats: Stats) -> Self {
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
            stats,
            selected_tab: 0,
            diary_table_state: diary_table_state,
            watchlist_table_state: watchlist_table_state,
            poster_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_fetches: Arc::new(Mutex::new(HashSet::new())),
            diary_prefetch_cursor: 0,
            watchlist_prefetch_cursor: 0,
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

    fn render_text_block(
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

    /// Truncates `text` (appending "...") to the longest prefix that wraps to at most
    /// `max_lines` lines at the given `width`, using ratatui's own wrapping so it always
    /// matches what will actually render.
    fn fit_text(text: &str, width: u16, max_lines: u16) -> String {
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

    fn render_diary(&mut self, frame: &mut Frame, area: Rect) {
        let full_profile = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let diary_list_block = Block::bordered().title("Movies").style(Color::White);
        let inner = diary_list_block.inner(full_profile[0]);
        frame.render_widget(diary_list_block, full_profile[0]);
        Self::render_movie_table(frame, inner, &self.user.diary, &mut self.diary_table_state);

        let burst = if self.diary_prefetch_cursor == 0 {
            10
        } else {
            1
        };
        Self::prefetch_window(
            &self.user.diary,
            &mut self.diary_prefetch_cursor,
            burst,
            &self.poster_cache,
            &self.pending_fetches,
        );

        let selected = self
            .diary_table_state
            .selected()
            .and_then(|i| self.user.diary.get(i));
        Self::render_movie_panel(
            frame,
            full_profile[1],
            selected,
            &self.poster_cache,
            &self.pending_fetches,
        );
    }

    fn render_movie_panel(
        frame: &mut Frame,
        area: Rect,
        movie: Option<&Movie>,
        cache: &Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
        pending: &Arc<Mutex<HashSet<(String, i32)>>>,
    ) {
        let block = Block::bordered().title("Movie Info").style(Color::White);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let movie = match movie {
            Some(m) => m,
            None => {
                let paragraph = Paragraph::new("No movie selected").centered();
                frame.render_widget(paragraph, inner.centered_vertically(Constraint::Length(1)));
                return;
            }
        };

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(18), Constraint::Min(0)])
            .split(inner);

        let poster_area = sections[0].inner(Margin::new(0, 1));
        let poster_rect = poster_area.centered_horizontally(Constraint::Length(22));
        Self::ensure_fetch_started(movie, cache, pending);

        match cache
            .lock()
            .unwrap()
            .get(&(movie.name.clone(), movie.year))
            .cloned()
        {
            Some(Some(img)) => frame.render_widget(PosterWidget { img }, poster_rect),
            Some(None) => {
                frame.render_widget(Paragraph::new("No Poster Found").centered(), poster_rect)
            }
            None => {
                frame.render_widget(Paragraph::new("Loading poster...").centered(), poster_rect)
            }
        }

        let rating = match movie.rating {
            Some(r) => r.to_string(),
            None => String::from("-"),
        };

        let review = match movie.review.as_deref() {
            Some(review) => strip_html_tags(review),
            None => String::from("No review"),
        };

        let text_area = sections[1].inner(Margin::new(1, 1));

        let header = format!("{} ({})\nRating: {}", movie.name, movie.year, rating);
        let header_lines = Paragraph::new(header.clone())
            .wrap(Wrap { trim: true })
            .line_count(text_area.width) as u16;
        let body_budget = text_area.height.saturating_sub(header_lines + 1);
        let review = Self::fit_text(&review, text_area.width, body_budget);

        let text = format!("{header}\n\n{review}");
        let paragraph = Paragraph::new(text).centered().wrap(Wrap { trim: true });
        let line_count = paragraph.line_count(text_area.width) as u16;
        let text_rect = text_area.centered_vertically(Constraint::Length(line_count));
        frame.render_widget(paragraph, text_rect);
    }

    pub fn render_movie_table(
        frame: &mut Frame,
        area: Rect,
        m: &[Movie],
        table_state: &mut TableState,
    ) {
        let header = Row::new(["Title", "Year", "Rating"])
            .style(Style::new().bold())
            .bottom_margin(1);

        let rows: Vec<Row> = m
            .iter()
            .map(|movie| {
                let rating = match movie.rating {
                    Some(r) => r.to_string(),
                    None => String::from("-"),
                };
                Row::new([movie.name.clone(), movie.year.to_string(), rating])
            })
            .collect();
        let widths = [
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Percentage(10),
        ];
        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(1)
            .style(Color::White)
            .row_highlight_style(Style::new().on_black().bold())
            .column_highlight_style(Color::Gray)
            .cell_highlight_style(Style::new().reversed().light_blue())
            .highlight_symbol("🍿 ");

        frame.render_stateful_widget(table, area, table_state);
    }

    fn render_watchlist(&mut self, frame: &mut Frame, area: Rect) {
        let full_profile = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let diary_list_block = Block::bordered().title("Movies").style(Color::White);
        let inner = diary_list_block.inner(full_profile[0]);
        frame.render_widget(diary_list_block, full_profile[0]);
        Self::render_movie_table(
            frame,
            inner,
            &self.user.watchlist,
            &mut self.watchlist_table_state,
        );

        let burst = if self.diary_prefetch_cursor == 0 {
            10
        } else {
            1
        };
        Self::prefetch_window(
            &self.user.watchlist,
            &mut self.watchlist_prefetch_cursor,
            burst,
            &self.poster_cache,
            &self.pending_fetches,
        );

        let selected = self
            .watchlist_table_state
            .selected()
            .and_then(|i| self.user.watchlist.get(i));
        Self::render_movie_panel(
            frame,
            full_profile[1],
            selected,
            &self.poster_cache,
            &self.pending_fetches,
        );
    }

    fn render_visualizer(&mut self, frame: &mut Frame, area: Rect) {
        let w = Paragraph::new("visualizer")
            .block(Block::bordered().title("visualizer"))
            .centered()
            .style(Color::White);
        frame.render_widget(w, area);
    }

    fn render_homepage(&mut self, frame: &mut Frame, area: Rect) {
        let full_profile = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);
        let barchart_panel_width = LABELS.len() as u16 * (BAR_WIDTH + 1) - 1 + 2;
        let info_and_status = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(10),
                Constraint::Length(24),
                Constraint::Length(barchart_panel_width),
            ])
            .split(full_profile[0]);

        let profile_header = format!(
            "@{}\n{}\n📍 {}",
            self.user.profile.username, self.user.profile.pronoun, self.user.profile.location
        );
        Self::render_text_block(
            frame,
            info_and_status[0],
            "Info",
            Color::Rgb(255, 128, 0),
            &profile_header,
            &self.user.profile.bio,
        );

        let stats_text = format!(
            "Movies Seen: {}\n Watchlist Size: {}\nLikes: {}\n5 Star Ratings: {}",
            self.stats.watched_count,
            self.stats.watchlist_count,
            self.stats.likes,
            self.stats.ratings[10]
        );
        Self::render_text_block(
            frame,
            info_and_status[1],
            "Stats",
            Color::Rgb(0, 224, 84),
            &stats_text,
            "",
        );

        Self::render_vertical_barchart(frame, info_and_status[2], &self.stats.ratings);

        if !self.favorites.is_empty() {
            Self::render_favorites(frame, full_profile[1], &self.favorites);
        }
    }

    fn render_vertical_barchart(frame: &mut Frame, area: Rect, ratings: &[u64; 11]) {
        let bars: Vec<Bar> = LABELS
            .iter()
            .zip(ratings.iter())
            .map(|(&k, &v)| {
                Bar::with_label(k, v)
                    .style(Style::default().fg(Color::LightCyan))
                    .text_value("")
            })
            .collect();
        let block = Block::default()
            .title("⭐ Star Breakdown")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(64, 188, 244)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let content_width = bars.len() as u16 * (BAR_WIDTH + 1) - 1;
        let chart_rect = inner.centered_horizontally(Constraint::Length(content_width));

        let chart = BarChart::vertical(bars).bar_width(BAR_WIDTH);
        frame.render_widget(chart, chart_rect);
    }

    fn render_favorites(frame: &mut Frame, area: Rect, favorites: &[DynamicImage]) {
        let favorites_block = Block::bordered().title("Favorites").style(Color::White);
        let inner = favorites_block.inner(area);
        frame.render_widget(favorites_block, area);
        let n = favorites.len() as u32;
        let constraints = vec![Constraint::Ratio(1, n); n as usize];
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .margin(1)
            .spacing(2)
            .split(inner);

        for (idx, movie) in favorites.iter().enumerate() {
            let rect = layout[idx];
            let pw = PosterWidget { img: movie.clone() };
            frame.render_widget(pw, rect);
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

    fn quit(&mut self) {
        self.running = false;
    }

    fn prefetch_window(
        movies: &[Movie],
        cursor: &mut usize,
        count: usize,
        cache: &Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
        pending: &Arc<Mutex<HashSet<(String, i32)>>>,
    ) {
        let end = usize::min(*cursor + count, movies.len());
        for m in &movies[*cursor..end] {
            Self::ensure_fetch_started(m, cache, pending);
        }
        *cursor = end;
    }

    fn ensure_fetch_started(
        movie: &Movie,
        cache: &Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
        pending: &Arc<Mutex<HashSet<(String, i32)>>>,
    ) {
        let key = (movie.name.clone(), movie.year);
        if cache.lock().unwrap().contains_key(&key) {
            return;
        } else {
            if !pending.lock().unwrap().insert(key.clone()) {
                return;
            } else {
                let movie = movie.clone();
                let cache = Arc::clone(cache);
                let pending = Arc::clone(pending);
                let key = key.clone();
                spawn(move || {
                    let poster = match movie.get_poster() {
                        Ok(poster) => poster,
                        Err(e) => {
                            log::log_error(&format!("Failed to get poster: {e}"));
                            None
                        }
                    };
                    cache.lock().unwrap().insert(key.clone(), poster);
                    pending.lock().unwrap().remove(&key);
                });
            }
        }
    }
}
