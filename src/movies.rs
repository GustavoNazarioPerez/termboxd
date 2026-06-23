use image::{DynamicImage, Pixel, imageops::Triangle};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};
use std::env;
use tmdb_client::apis::client::APIClient;

#[derive(Default)]
pub struct Stats {
    pub watched_count: u32,
    pub watchlist_count: u32,
    pub likes: u32,
    pub ratings: [u64; 11],
}

#[derive(thiserror::Error, Debug)]
pub enum PosterError {
    #[error("tmdb error: {0}")]
    Tmdb(#[from] tmdb_client::Error),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("image decode error: {0}")]
    Image(#[from] image::ImageError),
    #[error("failed to parse year: {0}")]
    ParseYear(#[from] std::num::ParseIntError),
}

#[derive(Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Movie {
    pub date: String,
    pub name: String,
    pub year: i32,
    #[serde(rename = "Letterboxd URI")]
    pub uri: String,
    pub rating: Option<f32>,
    pub review: Option<String>,
}

impl Movie {
    fn get_poster_path(&self) -> Result<Option<String>, tmdb_client::Error> {
        let tmdb_api_key = env::var("TMDB_API_KEY").expect("ERROR: TMDB API key failure");
        let client = APIClient::new_with_api_key(tmdb_api_key);
        let res = client.search_api().get_search_movie_paginated(
            &self.name,
            Some(self.year),
            None,
            None,
            None,
            None,
            None,
        )?;
        let results = res.results.unwrap_or_default();

        // Explicitly return in the case of an empty movie search result
        if results.len() == 0 {
            return Ok(None);
        }

        let first = &results[0];
        let path = &first.poster_path;
        match path {
            Some(value) => Ok(Some(value.to_string())),
            None => Ok(None),
        }
    }

    pub fn get_poster(&self) -> Result<Option<DynamicImage>, PosterError> {
        let poster_path = match self.get_poster_path()? {
            Some(val) => val,
            None => return Ok(None),
        };
        let url = format!("https://image.tmdb.org/t/p/w500{}", poster_path);
        let response = reqwest::blocking::get(url)?;
        let bytes = response.bytes()?;
        let res = image::load_from_memory(&bytes)?;
        Ok(Some(res))
    }
}

pub fn fetch_title_year(uri: &str) -> Result<Option<(String, i32)>, PosterError> {
    let response = reqwest::blocking::get(uri)?;
    let body = response.text()?;

    let marker = "<meta property=\"og:title\" content=\"";
    let start = match body.find(marker) {
        Some(val) => val + marker.len(),
        None => return Ok(None),
    };
    let end = match body[start..].find('"') {
        Some(val) => start + val,
        None => return Ok(None),
    };

    let title = &body[start..end];
    let open = match title.rfind('(') {
        Some(val) => val,
        None => return Ok(None),
    };
    let close = match title.rfind(')') {
        Some(val) => val,
        None => return Ok(None),
    };

    let year: i32 = title[open + 1..close].parse()?;
    let name = title[..open].trim().to_string();
    Ok(Some((name, year)))
}

pub fn get_image_from_uri(uri: &str) -> Result<Option<DynamicImage>, PosterError> {
    let (name, year) = match fetch_title_year(uri)? {
        None => return Ok(None),
        Some((name, year)) => (name, year),
    };

    // Create a simple movie for the purpose of rendering.
    // This shouldn't be treated as a source of truth.
    let movie = Movie {
        date: String::from("Now"),
        name,
        year,
        uri: uri.to_string(),
        rating: None,
        review: None,
    };
    movie.get_poster()
}

pub struct PosterWidget {
    pub img: DynamicImage,
}

impl Widget for PosterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rgb = self
            .img
            .resize_exact(area.width as u32, area.height as u32 * 2, Triangle)
            .into_rgba8();
        let mut rows = rgb.rows();
        let mut r = 0;
        while let Some(top_row) = rows.next() {
            let mut c = 0;
            let merged = top_row.zip(
                rows.next()
                    .expect("ERROR: Failed to merge poster image rows"),
            );
            for (top, bottom) in merged {
                let fore = top.channels();
                let (fore_r, fore_g, fore_b) = (fore[0], fore[1], fore[2]);
                let back = bottom.channels();
                let (back_r, back_g, back_b) = (back[0], back[1], back[2]);
                buf[(area.x + c, area.y + r)]
                    .set_symbol("▀")
                    .set_fg(Color::Rgb(fore_r, fore_g, fore_b))
                    .set_bg(Color::Rgb(back_r, back_g, back_b));
                c = c + 1;
            }
            r = r + 1;
        }
    }
}
