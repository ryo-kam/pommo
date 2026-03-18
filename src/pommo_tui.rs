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

use crate::pommo_core::{PommoSession, PommoType};

#[derive(Debug)]
pub struct PommoTui {
    session: PommoSession,
    is_running: bool,
}

impl Default for PommoTui {
    fn default() -> Self {
        Self::new()
    }
}

impl PommoTui {
    pub fn new() -> Self {
        Self {
            session: PommoSession::new(),
            is_running: true,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.is_running {
            terminal.draw(|frame| self.draw(frame))?;

            if self.is_event_available() {
                self.handle_events().wrap_err("handling event failed")?;
            }
        }

        Ok(())
    }

    fn is_event_available(&self) -> bool {
        event::poll(Duration::from_millis(10)).unwrap_or(false)
    }

    fn draw(&mut self, frame: &mut Frame) {
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
            KeyCode::Char('s') => self.session.toggle_timer(),
            KeyCode::Char('n') => self.session.next_pommo(),
            _ => {}
        };

        Ok(())
    }

    fn exit(&mut self) {
        self.is_running = false;
    }
}

impl Widget for &mut PommoTui {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title = Line::from(" pommo ").bold();

        let instructions = Line::from(vec![
            " Start/Stop ".into(),
            "<S>".blue().bold(),
            " Next ".into(),
            "<N>".blue().bold(),
            " Quit ".into(),
            "<Q>".gray().bold(),
            " ".into(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let time_left = self.session.timer.get_time_left().as_secs();

        let mins_left = time_left / 60;
        let secs_left = time_left % 60;

        let pommo_type = match &self.session.current_pommo().pommo_type {
            PommoType::Break => "break",
            PommoType::Work => "work",
        };

        let main_text = Text::from(vec![
            Line::from(pommo_type),
            Line::from(format!("{:0>2}:{:0>2}", mins_left, secs_left)),
        ]);

        Paragraph::new(main_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
