use crate::pommo_tui::PommoTui;
use color_eyre::eyre::Result;

mod pommo_core;
mod pommo_tui;
mod timer;

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| PommoTui::new().run(terminal))?;

    Ok(())
}
