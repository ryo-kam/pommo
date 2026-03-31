use std::time::Duration;

use crate::timer::{Timer, TimerState};

const SHORT_BREAK: Pommo = Pommo {
    pommo_type: PommoType::Break,
    duration: Duration::from_mins(5),
};

const LONG_BREAK: Pommo = Pommo {
    pommo_type: PommoType::Break,
    duration: Duration::from_mins(30),
};

const WORK: Pommo = Pommo {
    pommo_type: PommoType::Work,
    duration: Duration::from_mins(25),
};

const POMMOS: &[Pommo] = &[
    WORK,
    SHORT_BREAK,
    WORK,
    SHORT_BREAK,
    WORK,
    SHORT_BREAK,
    WORK,
    LONG_BREAK,
];

pub struct Pommo {
    pub duration: Duration,
    pub pommo_type: PommoType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PommoType {
    Break,
    Work,
}

#[derive(Debug)]
pub struct PommoSession {
    pub timer: Timer,
    current_pommo_index: usize,
}

impl PommoSession {
    pub fn new() -> Self {
        Self {
            current_pommo_index: 0,
            timer: Timer::new(POMMOS[0].duration),
        }
    }

    pub fn current_pommo(&self) -> &'static Pommo {
        &POMMOS[self.current_pommo_index]
    }

    pub fn next_pommo(&mut self) {
        self.current_pommo_index = (self.current_pommo_index + 1) % POMMOS.len();
        self.timer = Timer::new(self.current_pommo().duration);
    }

    pub fn toggle_timer(&mut self) {
        match self.timer.check_time() {
            (_, TimerState::Ready | TimerState::Paused) => self.timer.start(),
            (_, TimerState::Running) => self.timer.pause(),
            (_, TimerState::Completed) => {
                self.next_pommo();
                self.timer.start();
            }
        }
    }
}
