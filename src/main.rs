use std::io::{self, Write};

mod log;
mod movies;
mod parser;
mod ui;
mod user;

fn main() -> color_eyre::Result<()> {
    print!("Enter your username: ");
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let username = String::from(input.trim());
    let user = match user::create_user(username) {
        Ok(Some(user)) => user,
        Ok(None) => return Err(color_eyre::eyre::eyre!("User doesn't exist")),
        Err(_) => return Err(color_eyre::eyre::eyre!("Failed to create user")),
    };

    let favorites = user.fetch_favorite_posters();
    let stats = user.fetch_stats()?;

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = ui::App::new(user, favorites, stats).run(terminal);
    ratatui::restore();
    result
}
