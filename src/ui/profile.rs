use image::DynamicImage;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Bar, BarChart, Block, Borders},
};

use super::App;

const LABELS: [&str; 11] = [
    "0", ".5", "1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5", "5",
];
const BAR_WIDTH: u16 = 4;

impl App {
    pub(super) fn render_homepage(&mut self, frame: &mut Frame, area: Rect) {
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

        let diary = self.user.diary.lock().unwrap();
        let watchlist_count = self.user.watchlist.lock().unwrap().len();
        let likes = *self.user.likes.lock().unwrap();
        let mut ratings: [u64; 11] = [0; 11];
        for mov in diary.iter() {
            let rating = mov.rating.unwrap_or(-1.0);
            if rating >= 0.0 {
                ratings[(rating * 2.0) as usize] += 1;
            }
        }
        let stats_text = format!(
            "Movies Seen: {}\nWatchlist Size: {}\nLikes: {}\n5 Star Ratings: {}",
            diary.len(),
            watchlist_count,
            likes,
            ratings[10]
        );
        drop(diary);
        Self::render_text_block(
            frame,
            info_and_status[1],
            "Stats",
            Color::Rgb(0, 224, 84),
            &stats_text,
            "",
        );
        Self::render_vertical_barchart(frame, info_and_status[2], &ratings);

        let favorites = self.favorites.lock().unwrap();
        if !favorites.is_empty() {
            Self::render_favorites(frame, full_profile[1], &favorites);
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
        frame.render_widget(BarChart::vertical(bars).bar_width(BAR_WIDTH), chart_rect);
    }

    fn render_favorites(frame: &mut Frame, area: Rect, favorites: &[Option<DynamicImage>]) {
        use crate::movies::PosterWidget;
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

        for (idx, slot) in favorites.iter().enumerate() {
            let rect = layout[idx];
            match slot {
                Some(img) => frame.render_widget(PosterWidget { img: img.clone() }, rect),
                None => frame.render_widget(Block::bordered().dim(), rect),
            }
        }
    }
}
