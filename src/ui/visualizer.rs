use crate::movies::Movie;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Color,
    widgets::{Block, Paragraph},
};

use super::App;

impl App {
    pub(super) fn render_visualizer(&mut self, frame: &mut Frame, area: Rect) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let input_text = format!("{}_", self.visualizer_input);
        let input = Paragraph::new(input_text)
            .block(Block::bordered().title("Enter a movie title, then press Enter"))
            .style(Color::White);
        frame.render_widget(input, sections[0]);

        if self.visualizer_query.is_empty() {
            let hint =
                Paragraph::new("Type a movie title and press Enter to see its poster").centered();
            frame.render_widget(hint, sections[1].centered_vertically(Constraint::Length(1)));
            return;
        }

        let movie = Movie { name: self.visualizer_query.clone(), ..Default::default() };
        let poster_area = sections[1].inner(Margin::new(0, 1));
        let poster_rect = poster_area
            .centered_horizontally(Constraint::Length(30))
            .centered_vertically(Constraint::Length(25));
        Self::render_poster_status(frame, poster_rect, &movie, &self.poster_cache, &self.pending_fetches);
    }

    pub(super) fn on_visualizer_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Esc) => self.selected_tab = 0,
            (_, KeyCode::Enter) => self.visualizer_query = self.visualizer_input.clone(),
            (_, KeyCode::Backspace) => { self.visualizer_input.pop(); }
            (_, KeyCode::Char(c)) => self.visualizer_input.push(c),
            _ => {}
        }
    }
}
