mod timer;

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

use crate::timer::{Timer, TimerState};

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| App::new().run(terminal))?;

    Ok(())
}

#[derive(Debug)]
pub struct App {
    timer: Timer,
    is_running: bool,
}

impl App {
    fn new() -> Self {
        Self {
            timer: Timer::new(Duration::from_mins(5)),
            is_running: true,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.is_running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events().wrap_err("handling event failed")?;
        }

        Ok(())
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
            KeyCode::Char('s') => self.toggle_timer(),
            _ => {}
        };

        Ok(())
    }

    fn exit(&mut self) {
        self.is_running = false;
    }

    fn toggle_timer(&mut self) {
        match self.timer.get_timer_state() {
            TimerState::Paused => self.timer.start(),
            TimerState::Ticking { .. } => self.timer.pause(),
            TimerState::Finished => Ok(()),
        }
        .unwrap();
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title = Line::from(" pommo ").bold();

        let instructions = Line::from(vec![
            " Start/Stop ".into(),
            "<S>".blue().bold(),
            " Quit ".into(),
            "<Q>".gray().bold(),
            " ".into(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let time_left = self.timer.get_time_left().as_secs();

        let mins_left = time_left / 60;
        let secs_left = time_left % 60;

        let main_text = Text::from(vec![Line::from(vec![
            "Timer: ".into(),
            format!("{:0>2}:{:0>2}", mins_left, secs_left).into(),
        ])]);

        Paragraph::new(main_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
