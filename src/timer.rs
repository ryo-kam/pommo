use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerState {
    Ready,
    Running,
    Paused,
    Completed,
}

enum TimerCommand {
    Start,
    Pause,
}

#[derive(Debug)]
pub struct Timer {
    duration: Duration,

    state: TimerState,
    last_state_change_at: Instant,
    time_elapsed_previous_intervals: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Timer {
        Timer {
            duration,
            last_state_change_at: Instant::now(),
            time_elapsed_previous_intervals: Duration::ZERO,
            state: TimerState::Ready,
        }
    }

    pub fn check_time(&mut self) -> (Duration, TimerState) {
        let time_elapsed = self.get_time_elapsed();

        if time_elapsed >= self.duration {
            self.time_elapsed_previous_intervals = self.duration;
            self.set_state(TimerState::Completed);

            return (Duration::ZERO, self.state);
        }

        (self.duration - time_elapsed, self.state)
    }

    pub fn pause(&mut self) {
        self.send_command(TimerCommand::Pause);
    }

    pub fn start(&mut self) {
        self.send_command(TimerCommand::Start);
    }

    fn send_command(&mut self, command: TimerCommand) {
        if self.state == TimerState::Completed {
            return;
        }

        let time_elapsed = self.get_time_elapsed();

        if time_elapsed >= self.duration {
            self.time_elapsed_previous_intervals = self.duration;
            self.set_state(TimerState::Completed);
            return;
        }

        self.time_elapsed_previous_intervals = time_elapsed;

        match command {
            TimerCommand::Start => self.set_state(TimerState::Running),
            TimerCommand::Pause => self.set_state(TimerState::Paused),
        };
    }

    fn get_time_elapsed(&self) -> Duration {
        if self.state == TimerState::Running {
            self.time_elapsed_previous_intervals + self.last_state_change_at.elapsed()
        } else {
            self.time_elapsed_previous_intervals
        }
    }

    fn set_state(&mut self, state: TimerState) {
        self.state = state;
        self.last_state_change_at = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Instant};

    use super::*;

    /// an arbitrary acceptable error for difference in time
    const EPSILON: Duration = Duration::from_millis(1);

    const DURATION: Duration = Duration::from_millis(50);

    /// pause not long enough to complete the timer
    fn short_pause() {
        thread::sleep(Duration::from_millis(5));
    }

    /// long enough pause to complete the timer
    fn long_pause() {
        thread::sleep(Duration::from_millis(60));
    }

    #[test]
    fn instantiates_with_correct_values() {
        let timer = Timer::new(DURATION);

        assert_eq!(timer.duration, DURATION);
        assert!(matches!(timer.state, TimerState::Ready));
        assert!(timer.time_elapsed_previous_intervals.is_zero());
        assert!(Instant::now() - timer.last_state_change_at < EPSILON);
    }

    #[test]
    fn can_start_timer() {
        let mut timer = Timer::new(DURATION);

        timer.start();
        short_pause();

        let (_, timer_state) = timer.check_time();
        assert!(matches!(timer_state, TimerState::Running));
    }

    #[test]
    fn timer_finishes_when_run_for_duration() {
        let mut timer = Timer::new(DURATION);

        timer.start();
        long_pause();

        let (time_left, timer_state) = timer.check_time();
        assert!(matches!(timer_state, TimerState::Completed));
        assert!(time_left.is_zero());
    }

    #[test]
    fn can_pause_timer_when_ticking() {
        let mut timer = Timer::new(DURATION);

        timer.start();
        short_pause();
        timer.pause();

        let (_, timer_state) = timer.check_time();
        assert!(matches!(timer_state, TimerState::Paused));
    }

    #[test]
    fn time_elapsed_is_accurate_when_paused() {
        let mut timer = Timer::new(DURATION);

        let start_time_test = Instant::now();

        timer.start();
        short_pause();
        timer.pause();

        let time_left_test = DURATION.saturating_sub(start_time_test.elapsed());

        let (time_left_timer, _) = timer.check_time();
        let time_difference = time_left_timer.abs_diff(time_left_test);
        assert!(time_difference <= EPSILON);
    }

    #[test]
    fn time_elapsed_is_accurate_while_ticking() {
        let mut timer = Timer::new(DURATION);

        let start_time_test = Instant::now();

        timer.start();
        short_pause();

        let time_left_test = DURATION.saturating_sub(start_time_test.elapsed());

        let (time_left_timer, _) = timer.check_time();
        let time_difference = time_left_timer.abs_diff(time_left_test);
        assert!(time_difference <= EPSILON);
    }

    #[test]
    fn time_elapsed_is_accurate_when_paused_and_unpaused_repeatedly() {
        let mut timer = Timer::new(DURATION);

        let mut time_elapsed_test = Duration::ZERO;

        for _ in 0..5 {
            let start_time_test = Instant::now();

            timer.start();
            short_pause();
            timer.pause();

            time_elapsed_test += start_time_test.elapsed();

            short_pause();
        }

        let time_left_test = DURATION.saturating_sub(time_elapsed_test);

        let (time_left_timer, _) = timer.check_time();
        let time_difference = time_left_timer.abs_diff(time_left_test);
        assert!(time_difference <= EPSILON);
    }
}
