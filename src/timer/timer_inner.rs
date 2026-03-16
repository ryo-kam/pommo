use super::timer_shared::*;
use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, TryRecvError, channel},
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
    pub time_elapsed: Arc<Mutex<Duration>>,
    pub timer_state: Arc<Mutex<TimerState>>,
}

// use a result so we can use ? syntax
// Ok() returns the new timer state
// Err() means timer is finished
#[derive(Debug)]
enum TickResult {
    Continue,
    Paused,
    Finished,
}

impl TimerInner {
    pub fn new(duration: Duration) -> (Self, Sender<TimerCommand>) {
        let (command_tx, command_rx) = channel::<TimerCommand>();

        let time_elapsed = Arc::new(Mutex::new(Duration::ZERO));
        let timer_state = Arc::new(Mutex::new(TimerState::Paused));

        (
            Self {
                command_rx,
                duration,
                timer_state,
                time_elapsed,
                time_elapsed_previous_intervals: Duration::ZERO,
            },
            command_tx,
        )
    }

    pub fn run(mut self) {
        loop {
            let tick_result = self.tick().unwrap();

            match tick_result {
                TickResult::Continue => thread::sleep(LOOP_TIME),
                TickResult::Paused => continue,
                TickResult::Finished => break,
            }
        }
    }

    fn tick(&mut self) -> Result<TickResult, ()> {
        // store as local variable so it doesn't need to acquire mutex lock multiple times
        let current_state = self.get_state();

        let command_result = match current_state {
            // check for next command but don't wait when timer is ticking
            TimerState::Ticking { .. } => self.check_for_next_command()?,
            // block and wait for the next command when timer is paused
            TimerState::Paused => Some(self.wait_for_next_command()?),
            // exit without checking for a new message when the timer is finished
            TimerState::Finished => {
                return Ok(TickResult::Finished);
            }
        };

        // handle state transition
        let tick_result = match (current_state, command_result) {
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
            (TimerState::Ticking { start_time }, _) => self.increment_timer(start_time),
            // paused -> ticking
            (
                TimerState::Paused,
                Some(TimerCommand {
                    command_type: TimerCommandType::Start,
                    invoked_at,
                }),
            ) => {
                self.set_state(TimerState::Ticking {
                    start_time: invoked_at,
                });

                TickResult::Continue
            }
            // paused | finished -> finished
            (
                TimerState::Paused | TimerState::Finished,
                Some(TimerCommand {
                    command_type: TimerCommandType::End,
                    ..
                }),
            ) => {
                self.set_state(TimerState::Finished);

                TickResult::Finished
            }
            // other transitions shouldn't really happen
            (_, _) => TickResult::Paused,
        };

        Ok(tick_result)
    }

    fn wait_for_next_command(&self) -> Result<TimerCommand, ()> {
        match self.command_rx.recv() {
            Ok(command) => Ok(command),
            _ => Err(()),
        }
    }

    fn check_for_next_command(&self) -> Result<Option<TimerCommand>, ()> {
        match self.command_rx.try_recv() {
            Ok(command) => Ok(Some(command)),
            Err(TryRecvError::Empty) => Ok(None),
            _ => Err(()),
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

    fn increment_timer(&self, current_interval_start_time: Instant) -> TickResult {
        let time_elapsed_this_interval = current_interval_start_time.elapsed();

        let (_, finished) = self.add_time_elapsed(time_elapsed_this_interval);

        if finished {
            return TickResult::Finished;
        } else {
            return TickResult::Continue;
        }
    }

    fn pause_timer(
        &mut self,
        current_interval_start_time: Instant,
        invoked_at: Instant,
    ) -> TickResult {
        let time_elapsed_this_interval = invoked_at.duration_since(current_interval_start_time);

        let (new_total_time_elapsed, finished) = self.add_time_elapsed(time_elapsed_this_interval);

        if finished {
            return TickResult::Finished;
        } else {
            self.time_elapsed_previous_intervals = new_total_time_elapsed;
            self.set_state(TimerState::Paused);
            return TickResult::Paused;
        }
    }

    fn end_timer(
        &mut self,
        current_interval_start_time: Instant,
        invoked_at: Instant,
    ) -> TickResult {
        let time_elapsed_this_interval = invoked_at.duration_since(current_interval_start_time);

        self.add_time_elapsed(time_elapsed_this_interval);
        self.set_state(TimerState::Finished);

        return TickResult::Finished;
    }

    fn add_time_elapsed(&self, time_elapsed: Duration) -> (Duration, bool) {
        let mut current_time_elapsed = self.time_elapsed.lock().unwrap();
        *current_time_elapsed = self.time_elapsed_previous_intervals + time_elapsed;

        let total_time_elapsed = current_time_elapsed.clone();

        // drop to release mutex lock asap
        drop(current_time_elapsed);

        let mut finished = false;

        if total_time_elapsed >= self.duration {
            self.set_state(TimerState::Finished);
            finished = true;
        }

        (total_time_elapsed, finished)
    }
}
