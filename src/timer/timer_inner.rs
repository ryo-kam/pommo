use super::timer_shared::*;
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

// the loop pause duration when ticking
const LOOP_TIME: Duration = Duration::from_millis(10);

pub struct TimerInner {
    command_rx: Receiver<TimerCommand>,

    duration: Duration,
    time_elapsed_previous_intervals: Duration,
    time_elapsed: Arc<Mutex<Duration>>,
    timer_state: Arc<Mutex<TimerState>>,
}

impl TimerInner {
    pub fn new(
        duration: Duration,
        command_rx: Receiver<TimerCommand>,
        time_elapsed: Arc<Mutex<Duration>>,
        timer_state: Arc<Mutex<TimerState>>,
    ) -> Self {
        Self {
            command_rx,
            duration,
            timer_state,
            time_elapsed,
            time_elapsed_previous_intervals: Duration::ZERO,
        }
    }

    pub fn run(mut self) {
        loop {
            // store as local variable so it doesn't need to acquire mutex lock multiple times
            let current_state = self.get_state();

            let command_result = match current_state {
                // check for next command but don't wait when timer is ticking
                TimerState::Ticking { .. } => match self.command_rx.try_recv() {
                    Ok(command) => Some(command),
                    Err(TryRecvError::Empty) => None,
                    _ => break,
                },
                // block and wait for the next command when timer is paused
                TimerState::Paused => match self.command_rx.recv() {
                    Ok(command) => Some(command),
                    _ => break,
                },
                // exit without checking for a new message when the timer is finished
                TimerState::Finished => break,
            };

            // handle commands and ticking
            match (current_state, command_result) {
                // ticking -> paused
                (
                    TimerState::Ticking { start_time },
                    Some(TimerCommand {
                        command_type: TimerCommandType::Pause,
                        invoked_at,
                    }),
                ) => self.pause_timer(start_time, invoked_at),
                // ticking -> finished
                (
                    TimerState::Ticking { start_time },
                    Some(TimerCommand {
                        command_type: TimerCommandType::End,
                        invoked_at,
                    }),
                ) => self.end_timer(start_time, invoked_at),
                // ticking -> ticking
                (TimerState::Ticking { start_time }, _) => self.tick_timer(start_time),
                // paused -> ticking
                (
                    TimerState::Paused,
                    Some(TimerCommand {
                        command_type: TimerCommandType::Start,
                        invoked_at,
                    }),
                ) => self.set_state(TimerState::Ticking {
                    start_time: invoked_at,
                }),
                // paused | finished -> finished
                (
                    TimerState::Paused | TimerState::Finished,
                    Some(TimerCommand {
                        command_type: TimerCommandType::End,
                        ..
                    }),
                ) => {
                    self.set_state(TimerState::Finished);

                    break;
                }
                // other transitions shouldn't really happen
                (_, _) => {}
            }

            match self.get_state() {
                TimerState::Ticking { .. } => thread::sleep(LOOP_TIME),
                TimerState::Paused => continue,
                TimerState::Finished => break,
            }
        }
    }

    fn get_state(&self) -> TimerState {
        let timer_state = self.timer_state.lock().unwrap();
        timer_state.clone()
    }

    fn set_state(&self, timer_state: TimerState) {
        let mut current_timer_state = self.timer_state.lock().unwrap();
        *current_timer_state = timer_state;
    }

    fn get_time_elapsed(&self) -> Duration {
        let time_elapsed = self.time_elapsed.lock().unwrap();
        time_elapsed.clone()
    }

    fn tick_timer(&self, current_interval_start_time: Instant) {
        let time_elapsed_this_interval = current_interval_start_time.elapsed();

        self.add_time_elapsed(time_elapsed_this_interval);
    }

    fn pause_timer(&mut self, current_interval_start_time: Instant, invoked_at: Instant) {
        let time_elapsed_this_interval = invoked_at.duration_since(current_interval_start_time);

        let new_total_time_elapsed = self.add_time_elapsed(time_elapsed_this_interval);

        if self.get_state() != TimerState::Finished {
            self.set_state(TimerState::Paused);
            self.time_elapsed_previous_intervals = new_total_time_elapsed;
        }
    }

    fn end_timer(&mut self, current_interval_start_time: Instant, invoked_at: Instant) {
        let time_elapsed_this_interval = invoked_at.duration_since(current_interval_start_time);

        self.add_time_elapsed(time_elapsed_this_interval);

        self.set_state(TimerState::Finished);
    }

    fn add_time_elapsed(&self, time_elapsed: Duration) -> Duration {
        let mut current_time_elapsed = self.time_elapsed.lock().unwrap();
        *current_time_elapsed += time_elapsed;

        let total_time_elapsed = current_time_elapsed.clone();

        // drop to release mutex lock asap
        drop(current_time_elapsed);

        if total_time_elapsed >= self.duration {
            self.set_state(TimerState::Finished);
        }

        total_time_elapsed
    }

    fn handle_next_command(&self) -> Result<Option<TimerCommand>, ()> {
        let current_state = self.get_state();

        match current_state {
            // check for next command but don't wait when timer is ticking
            TimerState::Ticking { .. } => match self.command_rx.try_recv() {
                Ok(command) => Ok(Some(command)),
                Err(TryRecvError::Empty) => Ok(None),
                _ => Err(()),
            },
            // block and wait for the next command when timer is paused
            TimerState::Paused => match self.command_rx.recv() {
                Ok(command) => Ok(Some(command)),
                _ => Err(()),
            },
            // exit without checking for a new message when the timer is finished
            TimerState::Finished => Err(()),
        }
    }
}
