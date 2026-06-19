use crate::movies::Movie;
use csv;
use std::collections::HashMap;
use std::fs::File;

fn get_deserialized_records(username: &String, filename: &str) -> csv::Reader<File> {
    let path = format!("src/data/{username}/{filename}");
    let rdr_result = csv::Reader::from_path(path);
    let rdr = rdr_result.expect("Failed to open {path}");
    rdr
}

pub fn create_watchlist(username: &String) -> Vec<Movie> {
    let mut rdr = get_deserialized_records(username, "watchlist.csv");
    let mut watchlist = Vec::new();
    for result in rdr.deserialize() {
        let row: Movie = result.expect("Failed to deserialize record");
        watchlist.push(row);
    }
    watchlist
}

pub fn create_diary(username: &String) -> Vec<Movie> {
    // get all of the watched movies
    let mut rdr = get_deserialized_records(username, "diary.csv");
    let mut diary_map: HashMap<(i32, String), Movie> = HashMap::new();
    for result in rdr.deserialize() {
        let row: Movie = result.expect("Failed to deserialize record");
        diary_map.insert((row.year, row.name.clone()), row);
    }

    // Merge in the reviews
    let mut rdr = get_deserialized_records(username, "reviews.csv");
    for result in rdr.deserialize() {
        let row: Movie = result.expect("Failed to deserialize record");
        diary_map.insert((row.year, row.name.clone()), row);
    }

    let mut diary = Vec::new();
    for (_, m) in diary_map {
        diary.push(m);
    }
    diary
}
