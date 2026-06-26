use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crate::movies::{Movie, PosterWidget, strip_html_tags};
use image::DynamicImage;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    widgets::{Block, Paragraph, Row, Table, TableState, Wrap},
};

use super::App;

impl App {
    pub(super) fn render_diary(&mut self, frame: &mut Frame, area: Rect) {
        let full_profile = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let diary_list_block = Block::bordered().title("Movies").style(Color::White);
        let inner = diary_list_block.inner(full_profile[0]);
        frame.render_widget(diary_list_block, full_profile[0]);
        let diary_arc = Arc::clone(&self.user.diary);
        let diary = diary_arc.lock().unwrap();
        Self::render_movie_table(frame, inner, &diary, &mut self.diary_table_state);

        let burst = if self.diary_prefetch_cursor == 0 { 10 } else { 1 };
        Self::prefetch_window(
            &diary,
            &mut self.diary_prefetch_cursor,
            burst,
            &self.poster_cache,
            &self.pending_fetches,
        );

        let selected = self.diary_table_state.selected().and_then(|i| diary.get(i));
        Self::render_movie_panel(frame, full_profile[1], selected, &self.poster_cache, &self.pending_fetches);
    }

    pub(super) fn render_watchlist(&mut self, frame: &mut Frame, area: Rect) {
        let full_profile = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let list_block = Block::bordered().title("Movies").style(Color::White);
        let inner = list_block.inner(full_profile[0]);
        frame.render_widget(list_block, full_profile[0]);
        let watchlist_arc = Arc::clone(&self.user.watchlist);
        let watchlist = watchlist_arc.lock().unwrap();
        Self::render_movie_table(frame, inner, &watchlist, &mut self.watchlist_table_state);

        let burst = if self.watchlist_prefetch_cursor == 0 { 10 } else { 1 };
        Self::prefetch_window(
            &watchlist,
            &mut self.watchlist_prefetch_cursor,
            burst,
            &self.poster_cache,
            &self.pending_fetches,
        );

        let selected = self.watchlist_table_state.selected().and_then(|i| watchlist.get(i));
        Self::render_movie_panel(frame, full_profile[1], selected, &self.poster_cache, &self.pending_fetches);
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
        Self::render_poster_status(frame, poster_rect, movie, cache, pending);

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

    pub(super) fn render_poster_status(
        frame: &mut Frame,
        area: Rect,
        movie: &Movie,
        cache: &Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
        pending: &Arc<Mutex<HashSet<(String, i32)>>>,
    ) {
        Self::ensure_fetch_started(movie, cache, pending);
        match cache.lock().unwrap().get(&(movie.name.clone(), movie.year)).cloned() {
            Some(Some(img)) => frame.render_widget(PosterWidget { img }, area),
            Some(None) => frame.render_widget(Paragraph::new("No Poster Found").centered(), area),
            None => frame.render_widget(Paragraph::new("Loading poster...").centered(), area),
        }
    }

    fn ensure_fetch_started(
        movie: &Movie,
        cache: &Arc<Mutex<HashMap<(String, i32), Option<DynamicImage>>>>,
        pending: &Arc<Mutex<HashSet<(String, i32)>>>,
    ) {
        let key = (movie.name.clone(), movie.year);
        if cache.lock().unwrap().contains_key(&key) {
            return;
        }
        if !pending.lock().unwrap().insert(key.clone()) {
            return;
        }
        let movie = movie.clone();
        let cache = Arc::clone(cache);
        let pending = Arc::clone(pending);
        spawn(move || {
            let poster = match movie.get_poster() {
                Ok(p) => p,
                Err(e) => {
                    crate::log::log_error(&format!("Failed to get poster: {e}"));
                    None
                }
            };
            cache.lock().unwrap().insert(key.clone(), poster);
            pending.lock().unwrap().remove(&key);
        });
    }
}
