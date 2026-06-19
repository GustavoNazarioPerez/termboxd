use env;
use image::{DynamicImage, Pixel, imageops::Triangle};
use tmdb_client::apis::client::APIClient;

#[derive(Default, serde::Deserialize)]
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
    fn get_poster_path(&self) -> String {
        let tmdb_api_key = env::var("TMDB_API_KEY").expect("ERROR: TMDB API key failure");
        let client = APIClient::new_with_api_key(tmdb_api_key);
        let res = client
            .search_api()
            .get_search_movie_paginated(&self.name, Some(self.year), None, None, None, None, None)
            .expect("ERROR: Failed to search movie with TMDB");
        let results = res.results.unwrap_or_default();
        let first = &results[0];
        let path = &first.poster_path;
        match path {
            Some(value) => value.to_string(),
            None => String::from(""),
        }
    }

    fn get_poster(&self) -> DynamicImage {
        let poster_path = self.get_poster_path();
        let url = format!("https://image.tmdb.org/t/p/original{}", poster_path);
        let response = reqwest::blocking::get(url);
        let bytes = response
            .expect("ERROR: Failed to fetch poster from IMDB")
            .bytes()
            .expect("ERROR: Failed decoding poster image");
        image::load_from_memory(&bytes).expect("ERROR: Failed to load image from memory")
    }

    pub fn visualize_poster(&self) {
        let img = self.get_poster();
        let rgb = img.resize(48, 48, Triangle).into_rgba8();
        let mut rows = rgb.rows();
        while let Some(top_row) = rows.next() {
            let merged = top_row.zip(rows.next().expect("Error"));
            for (top, bottom) in merged {
                let fore = top.channels();
                let (fore_r, fore_g, fore_b) = (fore[0], fore[1], fore[2]);
                let back = bottom.channels();
                let (back_r, back_g, back_b) = (back[0], back[1], back[2]);
                print!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    fore_r, fore_g, fore_b, back_r, back_g, back_b
                );
                print!("\x1b[0m",);
            }

            println!("");
        }
    }
}
