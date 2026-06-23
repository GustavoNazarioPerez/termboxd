use crate::log;
use crate::movies::{Movie, Stats, get_image_from_uri};
use crate::parser::{count_records, create_diary, create_watchlist, get_csv_reader};
use image::DynamicImage;
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
    pub diary: Vec<Movie>,
    pub watchlist: Vec<Movie>,
}

impl User {
    pub fn fetch_favorite_posters(&self) -> Vec<DynamicImage> {
        let favorites: Vec<&str> = self
            .profile
            .favorites
            .split(", ")
            .filter(|s| !s.is_empty())
            .collect();
        let mut fav_posters = Vec::new();
        thread::scope(|s| {
            let handles: Vec<_> = favorites
                .iter()
                .map(|uri| s.spawn(|| get_image_from_uri(uri)))
                .collect();
            for handle in handles {
                match handle.join() {
                    Ok(Ok(Some(img))) => fav_posters.push(img),
                    Ok(Ok(None)) => log::log_error("No poster found for a favorite"),
                    Ok(Err(e)) => log::log_error(&format!("Failed to fetch favorite poster: {e}")),
                    Err(_) => log::log_error("A favorite poster worker thread panicked"),
                }
            }
        });
        fav_posters
    }

    pub fn fetch_stats(&self) -> Result<Stats, csv::Error> {
        let watched_count = if !self.diary.is_empty() {
            self.diary.len() as u32
        } else {
            count_records(&self.profile.username, "watched.csv")? as u32
        };

        let watchlist_count = if !self.watchlist.is_empty() {
            self.watchlist.len() as u32
        } else {
            count_records(&self.profile.username, "watchlist.csv")? as u32
        };

        let likes = count_records(&self.profile.username, "likes/films.csv")? as u32;
        let mut ratings: [u64; 11] = [0; 11];
        for mov in &self.diary {
            let rating = mov.rating.unwrap_or(-1.0);
            if rating != -1.0 {
                ratings[(rating * 2.0) as usize] += 1
            }
        }

        Ok(Stats {
            watched_count,
            watchlist_count,
            likes,
            ratings,
        })
    }
}

pub fn create_user(username: String) -> Result<Option<User>, csv::Error> {
    let mut rdr = get_csv_reader(&username, "profile.csv")?;
    let diary = create_diary(&username)?;
    let watchlist = create_watchlist(&username)?;
    match rdr.deserialize::<Profile>().next() {
        Some(profile) => Ok(Some(User {
            profile: profile?,
            diary,
            watchlist,
        })),
        None => Ok(None),
    }
}
