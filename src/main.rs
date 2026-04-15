use color_eyre::eyre::Result;

use crate::pommo_tui::run_pommo;

mod notifications;
mod pommo_core;
mod pommo_tui;
mod timer;

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(run_pommo)?;

    Ok(())
}
