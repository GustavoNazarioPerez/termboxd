mod log;
mod movies;
mod parser;

fn main() {
    let username = String::from("funnyguygus");

    let diary = parser::create_diary(&username).expect("Failed to create diary");
    let watchlist = parser::create_watchlist(&username).expect("Failed to create watchlist");

    println!("diary: {} watchlist: {}", diary.len(), watchlist.len());
}
