use ratatui::{prelude::*, widgets::Paragraph};

use crate::{
    pommo_core::{POMMOS, PommoType},
    pommo_tui::PommoAppState,
};

pub struct ListWidget;

impl StatefulWidget for ListWidget {
    type State = PommoAppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let vertically_centered_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(8),
                Constraint::Fill(1),
            ])
            .split(area)[1];

        let checkboxes: Vec<Line<'_>> = (0..8)
            .map(|i| {
                let checkbox = match i {
                    _ if i < state.session.current_pommo_index => "[x]",
                    _ if i == state.session.current_pommo_index => "[o]",
                    _ => "[ ]",
                };

                let colour = match POMMOS[i].pommo_type {
                    PommoType::Break => Color::Green,
                    PommoType::Work => Color::Red,
                };

                Line::from(checkbox.bold()).style(colour)
            })
            .collect();

        Paragraph::new(Text::from(checkboxes))
            .centered()
            .render(vertically_centered_area, buf);
    }
}
