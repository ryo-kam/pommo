mod timer_inner;
mod timer_shared;

pub use timer_inner::*;
pub use timer_shared::*;

use std::{
    sync::{
        Arc, Mutex,
        mpsc::{SendError, Sender},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Cache<TData: Clone> {
    stale_time: Duration,
    data: TData,
    last_retrieved: Instant,
}

impl<TData: Clone> Cache<TData> {
    fn new(data: TData) -> Self {
        Self {
            stale_time: Duration::from_millis(500),
            data,
            last_retrieved: Instant::now(),
        }
    }

    fn is_stale(&self) -> bool {
        self.last_retrieved.elapsed() >= self.stale_time
    }

    fn set_data(&mut self, data: &TData) {
        self.data = data.clone();
        self.last_retrieved = Instant::now();
    }

    fn get_data(&self) -> TData {
        self.data.clone()
    }
}

#[derive(Debug)]
pub struct Timer {
    command_tx: Sender<TimerCommand>,

    duration: Duration,
    time_elapsed_cache: Cache<Duration>,
    timer_state_cache: Cache<TimerState>,
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

impl Timer {
    pub fn new(duration: Duration) -> Timer {
        let (timer_inner, command_tx) = TimerInner::new(duration);

        let timer = Timer {
            command_tx,
            duration,
            time_elapsed_cache: Cache::new(timer_inner.time_elapsed.lock().unwrap().clone()),
            timer_state_cache: Cache::new(timer_inner.timer_state.lock().unwrap().clone()),
            time_elapsed: timer_inner.time_elapsed.clone(),
            timer_state: timer_inner.timer_state.clone(),
        };

        thread::spawn(move || timer_inner.run());

        timer
    }

    pub fn get_timer_state(&mut self) -> TimerState {
        if self.timer_state_cache.is_stale() {
            let timer_state = self.timer_state.lock().unwrap().clone();

            self.timer_state_cache.set_data(&timer_state);

            return timer_state;
        } else {
            return self.timer_state_cache.get_data();
        }
    }

    pub fn get_time_left(&mut self) -> Duration {
        let time_elapsed = if self.time_elapsed_cache.is_stale() {
            let time_elapsed = self.time_elapsed.lock().unwrap().clone();

            self.time_elapsed_cache.set_data(&time_elapsed);

            time_elapsed
        } else {
            self.time_elapsed_cache.get_data()
        };

        let duration = self.duration;
        duration.saturating_sub(time_elapsed)
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
        if *self.timer_state.lock().unwrap() == TimerState::Finished {
            return Ok(());
        }

        self.command_tx.send(TimerCommand::new(command_type))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    const LOOP_TIME: Duration = Duration::from_millis(10);

    /// an arbitrary acceptable error for difference in time
    const EPSILON: Duration = Duration::from_micros(100);

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
    fn instantiates_with_correct_values() {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let duration_timer = timer.duration;
        assert_eq!(duration_timer, duration);

        let timer_state = timer.timer_state.lock().unwrap();
        assert!(matches!(*timer_state, TimerState::Paused));

        let time_elapsed_timer = timer.time_elapsed.lock().unwrap();
        assert!(time_elapsed_timer.is_zero());
    }

    #[test]
    fn can_start_timer() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let mut timer = Timer::new(duration);

        timer.start()?;
        small_pause();

        let timer_state = timer.timer_state.lock().unwrap();
        assert!(matches!(*timer_state, TimerState::Ticking { .. }));

        Ok(())
    }

    #[test]
    fn timer_start_time_is_accurate() -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(2);
        let timer = Timer::new(duration);

        let start_time_test = Instant::now();

        timer.start()?;
        small_pause();

        let TimerState::Ticking { start_time } = *timer.timer_state.lock().unwrap() else {
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

        let timer_state = timer.timer_state.lock().unwrap();
        assert!(matches!(*timer_state, TimerState::Finished));

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

        let timer_state = timer.timer_state.lock().unwrap();
        assert!(matches!(*timer_state, TimerState::Paused));

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

        let timer_state = timer.timer_state.lock().unwrap();
        assert!(matches!(*timer_state, TimerState::Finished));

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

        let timer_state = timer.timer_state.lock().unwrap();
        assert!(matches!(*timer_state, TimerState::Finished));

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

        let time_elapsed_timer = timer.time_elapsed.lock().unwrap();
        let time_difference = time_elapsed_timer.abs_diff(time_elapsed_test);
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

        let time_elapsed_timer = timer.time_elapsed.lock().unwrap();
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

        let time_elapsed_timer = timer.time_elapsed.lock().unwrap();
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

        let time_elapsed_timer = timer.time_elapsed.lock().unwrap();
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
        let time_elapsed_timer = dbg!(timer.time_elapsed.lock().unwrap());
        let time_difference = dbg!(time_elapsed_timer.abs_diff(time_elapsed_test));
        assert!(time_difference <= EPSILON);

        Ok(())
    }

    
    #[test]
    fn make_sure_it_works()
    -> Result<(), SendError<TimerCommand>> {
        let duration = Duration::from_secs(20);
        let mut timer = Timer::new(duration);

        timer.start()?;
        
        for _ in 0..3 {
            println!("{:?}", timer.get_time_left());
            thread::sleep(Duration::from_secs(1));
        }

        timer.pause()?;

        thread::sleep(Duration::from_millis(200));
        dbg!(timer.get_timer_state());

        timer.start()?;
        
        for _ in 0..3 {
            println!("{:?}", timer.get_time_left());
            thread::sleep(Duration::from_secs(1));
        }

        timer.pause()?;

        thread::sleep(Duration::from_millis(200));
        dbg!(timer.get_timer_state());

        timer.start()?;
        
        for _ in 0..3 {
            println!("{:?}", timer.get_time_left());
            thread::sleep(Duration::from_secs(1));
        }

        timer.pause()?;

        Ok(())
    }
}
