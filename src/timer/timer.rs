use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, SendError, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

pub enum TimerCommandType {
    Start,
    Pause,
    End,
}

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

pub struct TimerInner {
    command_rx: mpsc::Receiver<TimerCommand>,

    duration: Duration,
    time_elapsed_previous_intervals: Duration,
    time_elapsed: Arc<Mutex<Duration>>,
    timer_state: Arc<Mutex<TimerState>>,
}

impl TimerInner {
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

pub struct Timer {
    command_tx: mpsc::Sender<TimerCommand>,

    duration: Duration,
    time_elapsed: Arc<Mutex<Duration>>,
    timer_state: Arc<Mutex<TimerState>>,
}

impl Drop for Timer {
    fn drop(&mut self) {
        // send an end command just in case it's still running
        // but don't check the result since it doesn't matter if it's already finished
        let _ = self.end();
    }
}

// the loop pause duration when ticking
const LOOP_TIME: Duration = Duration::from_millis(10);

impl Timer {
    pub fn get_timer_state(&self) -> TimerState {
        let timer_state = self.timer_state.lock().unwrap();
        timer_state.clone()
    }

    pub fn get_time_elapsed(&self) -> Duration {
        let time_elapsed = self.time_elapsed.lock().unwrap();
        time_elapsed.clone()
    }

    pub fn pause(&self) -> Result<(), SendError<TimerCommand>> {
        self.send_command(TimerCommandType::Pause)
    }

    pub fn start(&self) -> Result<(), SendError<TimerCommand>> {
        self.send_command(TimerCommandType::Start)
    }

    pub fn end(&self) -> Result<(), SendError<TimerCommand>> {
        self.send_command(TimerCommandType::End)
    }

    fn send_command(&self, command_type: TimerCommandType) -> Result<(), SendError<TimerCommand>> {
        if self.get_timer_state() == TimerState::Finished {
            return Ok(());
        }

        self.command_tx.send(TimerCommand::new(command_type))
    }

    pub fn new(duration: Duration) -> Timer {
        let (command_tx, command_rx) = mpsc::channel::<TimerCommand>();

        let timer_state = Arc::new(Mutex::new(TimerState::Paused));
        let time_elapsed = Arc::new(Mutex::new(Duration::ZERO));

        let mut timer_inner = TimerInner {
            command_rx,
            duration,
            time_elapsed_previous_intervals: Duration::ZERO,
            timer_state: timer_state.clone(),
            time_elapsed: time_elapsed.clone(),
        };

        thread::spawn(move || {
            loop {
                // store as local variable so it doesn't need to acquire mutex lock multiple times
                let current_state = timer_inner.get_state();

                let command_result = match current_state {
                    // check for next command but don't wait when timer is ticking
                    TimerState::Ticking { .. } => match timer_inner.command_rx.try_recv() {
                        Ok(command) => Some(command),
                        Err(TryRecvError::Empty) => None,
                        _ => break,
                    },
                    // block and wait for the next command when timer is paused
                    TimerState::Paused => match timer_inner.command_rx.recv() {
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
                    ) => timer_inner.pause_timer(start_time, invoked_at),
                    // ticking -> finished
                    (
                        TimerState::Ticking { start_time },
                        Some(TimerCommand {
                            command_type: TimerCommandType::End,
                            invoked_at,
                        }),
                    ) => timer_inner.end_timer(start_time, invoked_at),
                    // ticking -> ticking
                    (TimerState::Ticking { start_time }, _) => timer_inner.tick_timer(start_time),
                    // paused -> ticking
                    (
                        TimerState::Paused,
                        Some(TimerCommand {
                            command_type: TimerCommandType::Start,
                            invoked_at,
                        }),
                    ) => timer_inner.set_state(TimerState::Ticking {
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
                        timer_inner.set_state(TimerState::Finished);

                        break;
                    }
                    // other transitions shouldn't really happen
                    (_, _) => {}
                }

                match timer_inner.get_state() {
                    TimerState::Ticking { .. } => thread::sleep(LOOP_TIME),
                    TimerState::Paused => continue,
                    TimerState::Finished => break,
                }
            }
        });

        Timer {
            command_tx,
            duration,
            time_elapsed,
            timer_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// an arbitrary acceptable error for difference in time
    const EPSILON: Duration = Duration::from_millis(5);

    /// LOOP_TIME + EPSILON pause about the time it takes to complete a loop and process a command
    fn small_pause() {
        thread::sleep(LOOP_TIME + EPSILON);
    }

    /// 500ms pause to make sure it completes a few loops when ticking
    fn medium_pause() {
        thread::sleep(Duration::from_millis(500));
    }

    /// 2s pause to simulate it actually ticking for a few seconds
    fn long_pause() {
        thread::sleep(Duration::from_secs(2));
    }

    #[test]
    fn can_instantiate_timer_with_correct_field_values() {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let duration_timer = timer.duration;
        assert_eq!(duration_timer, duration);

        let timer_state = timer.get_timer_state();
        assert!(matches!(timer_state, TimerState::Paused));

        let time_elapsed_timer = timer.get_time_elapsed();
        assert!(time_elapsed_timer.is_zero());
    }

    #[test]
    fn can_start_timer() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        timer.start()?;
        small_pause();

        let timer_state = timer.get_timer_state();
        assert!(matches!(timer_state, TimerState::Ticking { .. }));

        Ok(())
    }

    #[test]
    fn timer_start_time_is_accurate() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let start_time_test = Instant::now();

        timer.start()?;
        small_pause();

        let TimerState::Ticking { start_time } = timer.get_timer_state() else {
            panic!("timer not started");
        };

        let time_difference = start_time.duration_since(start_time_test);
        assert!(time_difference <= EPSILON);

        Ok(())
    }

    #[test]
    fn timer_finishes_when_run_for_duration() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        timer.start()?;
        thread::sleep(duration + LOOP_TIME);

        let timer_state = timer.get_timer_state();
        assert!(matches!(timer_state, TimerState::Finished));

        Ok(())
    }

    #[test]
    fn can_pause_timer_when_ticking() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        timer.start()?;
        medium_pause();
        timer.pause()?;
        small_pause();

        let timer_state = timer.get_timer_state();
        assert!(matches!(timer_state, TimerState::Paused));

        Ok(())
    }

    #[test]
    fn can_end_timer_when_ticking() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        timer.start()?;
        medium_pause();
        timer.end()?;
        small_pause();

        let timer_state = timer.get_timer_state();
        assert!(matches!(timer_state, TimerState::Finished));

        Ok(())
    }

    #[test]
    fn can_end_timer_when_paused() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        timer.start()?;
        medium_pause();
        timer.pause()?;
        small_pause();
        timer.end()?;
        small_pause();

        let timer_state = timer.get_timer_state();
        assert!(matches!(timer_state, TimerState::Finished));

        Ok(())
    }

    #[test]
    fn time_elapsed_is_accurate_when_paused() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let start_time_test = Instant::now();

        timer.start()?;
        medium_pause();
        timer.pause()?;

        // get the time here to get the most accurate time between start and pause command being issued
        let time_elapsed_test = dbg!(start_time_test.elapsed());

        // wait for it to process the command
        small_pause();

        let time_elapsed_timer = dbg!(timer.get_time_elapsed());
        let time_difference = dbg!(time_elapsed_timer.abs_diff(time_elapsed_test));
        assert!(time_difference <= EPSILON);

        Ok(())
    }

    #[test]
    fn time_elapsed_is_accurate_when_ended_via_command() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let start_time_test = Instant::now();

        timer.start()?;
        medium_pause();
        timer.end()?;

        // get the time here to get the most accurate time between start and pause command being issued
        let time_elapsed_test = start_time_test.elapsed();

        // wait for it to process the command
        small_pause();

        let time_elapsed_timer = timer.get_time_elapsed();
        let time_difference = time_elapsed_timer.abs_diff(time_elapsed_test);
        assert!(time_difference <= EPSILON);

        Ok(())
    }

    #[test]
    fn time_elapsed_is_accurate_when_ended_from_hitting_target()
    -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        timer.start()?;
        thread::sleep(duration + LOOP_TIME);

        let time_elapsed_timer = timer.get_time_elapsed();
        let time_difference = time_elapsed_timer.abs_diff(duration);
        assert!(time_difference <= LOOP_TIME + EPSILON);

        Ok(())
    }

    #[test]
    fn time_elapsed_is_accurate_within_loop_time_while_ticking()
    -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let start_time_test = Instant::now();

        timer.start()?;
        medium_pause();

        let time_elapsed_timer = timer.get_time_elapsed();
        let time_elapsed_test = start_time_test.elapsed();
        let time_difference = time_elapsed_timer.abs_diff(time_elapsed_test);
        assert!(time_difference <= LOOP_TIME + EPSILON);

        Ok(())
    }

    #[test]
    fn time_elapsed_is_accurate_when_paused_and_unpaused_repeatedly()
    -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(20);
        let timer = Timer::new(duration);

        let mut time_elapsed_test = Duration::ZERO;

        for _ in 0..5 {
            let start_time_test = Instant::now();

            timer.start()?;
            long_pause();
            timer.pause()?;

            time_elapsed_test += start_time_test.elapsed();

            small_pause();
        }

        dbg!(time_elapsed_test);
        let time_elapsed_timer = dbg!(timer.get_time_elapsed());
        let time_difference = dbg!(time_elapsed_timer.abs_diff(time_elapsed_test));
        assert!(time_difference <= EPSILON);
        assert!(time_elapsed_timer.abs_diff(Duration::from_secs(15)) < EPSILON);

        Ok(())
    }
}
