mod app_widget;
mod list_widget;
mod timer_widget;

use std::time::Duration;

use color_eyre::eyre::{Context, Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use crate::{
    notifications::NotificationManger, pommo_core::PommoSession, pommo_tui::app_widget::AppWidget,
};

#[derive(Debug)]
pub struct PommoAppState {
    session: PommoSession,
    is_running: bool,
    notification_manager: NotificationManger,
}

impl Default for PommoAppState {
    fn default() -> Self {
        Self::new()
    }
}

impl PommoAppState {
    pub fn new() -> Self {
        Self {
            session: PommoSession::new(),
            is_running: true,
            notification_manager: NotificationManger::new(),
        }
    }
}

pub fn run_pommo(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut state = PommoAppState::new();

    while state.is_running {
        terminal.draw(|frame| frame.render_stateful_widget(AppWidget, frame.area(), &mut state))?;

        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    handle_key_event(&mut state, key_event)
                        .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}"))
                }
                _ => Ok(()),
            }
            .wrap_err("handling event failed")?;
        }
    }

    Ok(())
}

fn handle_key_event(state: &mut PommoAppState, key_event: event::KeyEvent) -> Result<()> {
    match key_event.code {
        KeyCode::Char('q') => state.is_running = false,
        KeyCode::Char('p') => bail!("panic button pressed!"),
        KeyCode::Char('s') => state.session.toggle_timer(),
        KeyCode::Char('n') => state.session.next_pommo(),
        _ => {}
    };

    Ok(())
}
