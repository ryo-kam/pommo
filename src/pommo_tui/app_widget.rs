use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{Block, Padding},
};

use crate::pommo_tui::{PommoAppState, list_widget::ListWidget, timer_widget::TimerWidget};

pub struct AppWidget;

impl StatefulWidget for AppWidget {
    type State = PommoAppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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
            .padding(Padding::uniform(1))
            .border_set(border::THICK);

        let inner_area = block.inner(area);

        block.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints(vec![
                Constraint::Length(5),
                Constraint::Fill(1),
                Constraint::Length(5),
            ])
            .split(inner_area);

        TimerWidget.render(layout[1], buf, state);
        ListWidget.render(layout[2], buf, state);
    }
}
