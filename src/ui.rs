use crate::movies::{PosterWidget, Stats};
use crate::user::User;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use image::DynamicImage;
use ratatui::symbols;
use ratatui::widgets::Tabs;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Bar, BarChart, Block, Borders, Paragraph, Wrap},
};

const LABELS: [&str; 11] = [
    "0", ".5", "1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5", "5",
];

#[derive(Default)]
pub struct App {
    running: bool,
    user: User,
    favorites: Vec<DynamicImage>,
    stats: Stats,
    selected_tab: usize,
}

impl App {
    pub fn new(user: User, favorites: Vec<DynamicImage>, stats: Stats) -> Self {
        Self {
            running: true,
            user,
            favorites,
            stats,
            selected_tab: 0,
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

    fn render_text_block(frame: &mut Frame, area: Rect, title: &str, color: Color, text: String) {
        let block = Block::bordered()
            .title(title)
            .border_style(Style::default().fg(color));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let paragraph = Paragraph::new(text).centered().wrap(Wrap { trim: true });
        let line_count = paragraph.line_count(inner.width) as u16;
        let text_rect = inner.centered_vertically(Constraint::Length(line_count));
        frame.render_widget(paragraph, text_rect);
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
        let w = Paragraph::new("diary")
            .block(Block::bordered().title("diary"))
            .centered()
            .style(Color::White);
        frame.render_widget(w, area);
    }

    fn render_watchlist(&mut self, frame: &mut Frame, area: Rect) {
        let w = Paragraph::new("watchlist")
            .block(Block::bordered().title("watchlist"))
            .centered()
            .style(Color::White);
        frame.render_widget(w, area);
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
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);
        let info_and_status = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(60),
            ])
            .split(full_profile[0]);

        let profile_text = format!(
            "@{}\n{}\n📍 {}\n\n{}",
            self.user.profile.username,
            self.user.profile.pronoun,
            self.user.profile.location,
            self.user.profile.bio
        );
        Self::render_text_block(
            frame,
            info_and_status[0],
            "Info",
            Color::Rgb(255, 128, 0),
            profile_text,
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
            stats_text,
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

        let content_width = bars.len() as u16 * (6 + 1) - 1;
        let chart_rect = inner.centered_horizontally(Constraint::Length(content_width));

        let chart = BarChart::vertical(bars).bar_width(6);
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
            .margin(2)
            .spacing(2)
            .split(inner);

        for (idx, movie) in favorites.iter().enumerate() {
            let rect = layout[idx];
            let pw = PosterWidget { img: movie.clone() };
            frame.render_widget(pw, rect);
        }
    }

    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
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
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
