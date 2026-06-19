mod movies;
mod parser;

fn main() {
    // Hardcoding this for now bc who cares (if you're reading this follow me)
    let username = String::from("funnyguygus");

    let diary = parser::create_diary(&username);
    println!("Diary");
    println!("============");
    for mov in diary {
        let title = mov.name;
        let rating = mov.rating.unwrap_or_default();
        let review = mov.review.unwrap_or_default();
        println!("{title}: {rating}");
        println!("{review}");
    }

    // construct watchlist: list of unseen movies
    println!("\nWatchlist");
    println!("============");
    let watchlist = parser::create_watchlist(&username);
    for mov in watchlist {
        let title = mov.name;
        println!("{title}");
    }
}
