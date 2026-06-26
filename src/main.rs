mod log;
mod movies;
mod parser;
mod ui;
mod user;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout},
    style::{Color, Stylize},
    widgets::{Block, Paragraph},
};

struct LoginScreen {
    input: String,
    submitted: Option<String>,
    running: bool,
}

impl LoginScreen {
    fn new() -> Self {
        Self {
            input: String::new(),
            submitted: None,
            running: false,
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<Option<String>> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.on_key(key);
                }
            }
        }
        Ok(self.submitted)
    }

    fn render(&self, frame: &mut Frame) {
        let [row] = Layout::vertical([Constraint::Length(3)])
            .flex(Flex::Center)
            .areas(frame.area());
        let [col] = Layout::horizontal([Constraint::Length(40)])
            .flex(Flex::Center)
            .areas(row);

        let input_text = format!("{}_", self.input);
        let widget = Paragraph::new(input_text)
            .block(
                Block::bordered()
                    .title(" termboxd ")
                    .title_bottom(" enter username, press Enter ")
                    .fg(Color::Yellow),
            )
            .fg(Color::White);
        frame.render_widget(widget, col);
    }

    fn on_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.running = false;
            }
            (_, KeyCode::Enter) => {
                if !self.input.is_empty() {
                    self.submitted = Some(self.input.clone());
                    self.running = false;
                }
            }
            (_, KeyCode::Backspace) => {
                self.input.pop();
            }
            (_, KeyCode::Char(c)) => self.input.push(c),
            _ => {}
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let username = LoginScreen::new().run(terminal)?;
    ratatui::restore();

    let username = match username {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = match user::create_user(username) {
        Ok(Some(user)) => user,
        Ok(None) => return Err(color_eyre::eyre::eyre!("User doesn't exist")),
        Err(_) => return Err(color_eyre::eyre::eyre!("Failed to create user")),
    };

    let favorites = user.fetch_favorite_posters();

    let terminal = ratatui::init();
    let result = ui::App::new(user, favorites).run(terminal);
    ratatui::restore();
    result
}
