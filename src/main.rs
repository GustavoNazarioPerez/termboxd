mod log;
mod movies;
mod parser;
mod ui;
mod user;

fn main() -> color_eyre::Result<()> {
    // Hardcoding this for now bc who cares (if you're reading this follow me)
    let user = match user::create_user(String::from("funnyguygus")) {
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
