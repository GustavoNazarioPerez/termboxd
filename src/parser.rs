use crate::movies::Movie;
use csv;
use std::collections::HashMap;
use std::fs::File;

pub fn count_records(username: &String, filename: &str) -> Result<usize, csv::Error> {
    let mut rdr = get_csv_reader(username, filename)?;
    Ok(rdr.records().count())
}

pub fn get_csv_reader(username: &String, filename: &str) -> Result<csv::Reader<File>, csv::Error> {
    let path = format!("src/data/{username}/{filename}");
    csv::Reader::from_path(path)
}

pub fn create_watchlist(username: &String) -> Result<Vec<Movie>, csv::Error> {
    let mut rdr = get_csv_reader(username, "watchlist.csv")?;
    let mut watchlist = Vec::new();
    for result in rdr.deserialize() {
        let row: Movie = result?;
        watchlist.push(row);
    }
    Ok(watchlist)
}

pub fn create_diary(username: &String) -> Result<Vec<Movie>, csv::Error> {
    let mut rdr = get_csv_reader(username, "diary.csv")?;
    let mut diary_map: HashMap<(i32, String), Movie> = HashMap::new();
    for result in rdr.deserialize() {
        let row: Movie = result?;
        diary_map.insert((row.year, row.name.clone()), row);
    }

    // Merge in the reviews
    let mut rdr = get_csv_reader(username, "reviews.csv")?;
    for result in rdr.deserialize() {
        let row: Movie = result?;
        diary_map.insert((row.year, row.name.clone()), row);
    }

    let mut diary = Vec::new();
    for (_, m) in diary_map {
        diary.push(m);
    }
    Ok(diary)
}
