use crate::log;
use crate::movies::{Movie, get_image_from_uri};
use crate::parser::{count_records, create_diary, create_watchlist, get_csv_reader};
use image::DynamicImage;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Profile {
    pub username: String,
    pub location: String,
    pub bio: String,
    pub pronoun: String,
    #[serde(rename = "Favorite Films")]
    pub favorites: String,
}

#[derive(Default)]
pub struct User {
    pub profile: Profile,
    pub diary: Arc<Mutex<Vec<Movie>>>,
    pub watchlist: Arc<Mutex<Vec<Movie>>>,
    pub likes: Arc<Mutex<u32>>,
}

impl User {
    pub fn fetch_favorite_posters(&self) -> Arc<Mutex<Vec<Option<DynamicImage>>>> {
        let uris: Vec<String> = self
            .profile
            .favorites
            .split(", ")
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        let n = uris.len();
        let posters: Arc<Mutex<Vec<Option<DynamicImage>>>> =
            Arc::new(Mutex::new(vec![None; n]));
        let username = self.profile.username.clone();
        for (idx, uri) in uris.into_iter().enumerate() {
            let posters = Arc::clone(&posters);
            let username = username.clone();
            thread::spawn(move || {
                posters.lock().unwrap()[idx] =
                    load_or_fetch_favorite(&username, &uri);
            });
        }
        posters
    }
}

fn favorite_cache_path(username: &str, uri: &str) -> PathBuf {
    let slug = uri.trim_end_matches('/').split('/').last().unwrap_or(uri);
    PathBuf::from(format!("src/data/{username}/favorites/{slug}.png"))
}

fn load_or_fetch_favorite(username: &str, uri: &str) -> Option<DynamicImage> {
    let cache_path = favorite_cache_path(username, uri);
    if cache_path.exists() {
        if let Ok(img) = image::open(&cache_path) {
            return Some(img);
        }
    }
    match get_image_from_uri(uri) {
        Ok(Some(img)) => {
            if let Some(parent) = cache_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = img.save(&cache_path);
            Some(img)
        }
        Ok(None) => None,
        Err(e) => {
            log::log_error(&format!("Failed to fetch favorite: {e}"));
            None
        }
    }
}

pub fn create_user(username: String) -> Result<Option<User>, csv::Error> {
    let mut rdr = get_csv_reader(&username, "profile.csv")?;
    match rdr.deserialize::<Profile>().next() {
        Some(Ok(profile)) => {
            let diary: Arc<Mutex<Vec<Movie>>> = Arc::new(Mutex::new(Vec::new()));
            let watchlist: Arc<Mutex<Vec<Movie>>> = Arc::new(Mutex::new(Vec::new()));
            let likes: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

            let d = Arc::clone(&diary);
            let u = username.clone();
            thread::spawn(move || match create_diary(&u) {
                Ok(v) => *d.lock().unwrap() = v,
                Err(e) => log::log_error(&format!("Failed to load diary: {e}")),
            });

            let w = Arc::clone(&watchlist);
            let u = username.clone();
            thread::spawn(move || match create_watchlist(&u) {
                Ok(v) => *w.lock().unwrap() = v,
                Err(e) => log::log_error(&format!("Failed to load watchlist: {e}")),
            });

            let l = Arc::clone(&likes);
            thread::spawn(move || match count_records(&username, "likes/films.csv") {
                Ok(n) => *l.lock().unwrap() = n as u32,
                Err(e) => log::log_error(&format!("Failed to count likes: {e}")),
            });

            Ok(Some(User { profile, diary, watchlist, likes }))
        }
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}
