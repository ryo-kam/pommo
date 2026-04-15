use ratatui::{prelude::*, widgets::Paragraph};

use crate::{pommo_core::PommoType, pommo_tui::PommoAppState, timer::TimerState};

pub struct TimerWidget;

impl StatefulWidget for TimerWidget {
    type State = PommoAppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (time_left, timer_state) = state.session.timer.check_time();

        if timer_state == TimerState::Completed {
            let current_pommo_type = state.session.current_pommo().pommo_type;

            state.notification_manager.notify(current_pommo_type);
        }

        let mins_left = time_left.as_secs() / 60;
        let secs_left = time_left.as_secs() % 60;

        let pommo_type = match &state.session.current_pommo().pommo_type {
            PommoType::Break => "break",
            PommoType::Work => "work",
        };

        let main_text = Text::from(vec![
            Line::from(pommo_type),
            Line::from(format!("{:0>2}:{:0>2}", mins_left, secs_left)),
        ]);

        let vertically_centered_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(2),
                Constraint::Fill(1),
            ])
            .split(area)[1];

        Paragraph::new(main_text)
            .centered()
            .render(vertically_centered_area, buf);
    }
}
