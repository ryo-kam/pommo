use std::time::Duration;

use color_eyre::eyre::{Context, Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| App::default().run(terminal))?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    duration: Duration,
    time_elapsed: Duration,
    exit: bool,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().wrap_err("handling event failed")?;
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: event::KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('p') => bail!("panic button pressed!"),
            _ => {}
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title = Line::from(" pommo ").bold();

        let instructions = Line::from(vec![
            " Start ".into(),
            "<R>".blue().bold(),
            " Pause ".into(),
            "<T>".red().bold(),
            " Quit ".into(),
            "<Q>".gray().bold(),
            " ".into(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let time_left = self.duration.abs_diff(self.time_elapsed).as_secs();
        let time_left_mins = time_left / 60;
        let time_left_secs = time_left % 60;

        let main_text = Text::from(vec![Line::from(vec![
            "Timer: ".into(),
            format!("{time_left_mins:0<2}:{time_left_secs:0<2}").yellow(),
        ])]);

        Paragraph::new(main_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
