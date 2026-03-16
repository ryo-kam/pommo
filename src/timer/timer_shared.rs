// types shared between the outer timer module and the inner timer

use std::time::Instant;

#[derive(Debug)]
pub enum TimerCommandType {
    Start,
    Pause,
    End,
}

#[derive(Debug)]
pub struct TimerCommand {
    pub command_type: TimerCommandType,
    pub invoked_at: Instant,
}

impl TimerCommand {
    pub fn new(command_type: TimerCommandType) -> Self {
        Self {
            command_type,
            invoked_at: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimerState {
    Ticking { start_time: Instant },
    Paused,
    Finished,
}
