mod movies;
mod parser;

fn main() {
    // // Hardcoding this for now bc who cares (if you're reading this follow me)
    let username = String::from("funnyguygus");

    let diary = parser::create_diary(&username);
    for mov in diary {
        mov.visualize_poster();
    }

    // // construct watchlist: list of unseen movies
    let watchlist = parser::create_watchlist(&username);
    for mov in watchlist {
        mov.visualize_poster();
    }
}
