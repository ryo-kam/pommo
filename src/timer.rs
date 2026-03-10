use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, TryRecvError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub enum TimerCommand {
    Start,
    Pause,
    End,
}

pub struct TimerInner {
    command_rx: mpsc::Receiver<TimerCommand>,

    duration: Duration,
    time_elapsed: Arc<Mutex<Duration>>,
    is_running: Arc<AtomicBool>,
}

pub struct Timer {
    thread_handle: JoinHandle<()>,
    command_tx: mpsc::Sender<TimerCommand>,

    duration: Duration,
    time_elapsed: Arc<Mutex<Duration>>,
    is_running: Arc<AtomicBool>,
}

impl Timer {
    pub fn new(duration: Duration) -> Timer {
        let (command_tx, command_rx) = mpsc::channel::<TimerCommand>();

        let is_running = Arc::new(AtomicBool::new(false));
        let time_elapsed = Arc::new(Mutex::new(Duration::ZERO));

        let timer_inner = TimerInner {
            command_rx,
            duration,
            is_running: is_running.clone(),
            time_elapsed: time_elapsed.clone(),
        };

        let thread_handle = thread::spawn(move || {
            loop {
                // deal with commands
                let command_result = if !timer_inner.is_running.load(Ordering::Relaxed) {
                    // block and wait for the next command if paused
                    let Ok(command) = timer_inner.command_rx.recv() else {
                        break;
                    };

                    Some(command)
                } else {
                    match timer_inner.command_rx.try_recv() {
                        Ok(command) => Some(command),
                        Err(TryRecvError::Empty) => None,
                        _ => break,
                    }
                };

                if let Some(command) = command_result {
                    match command {
                        TimerCommand::Start => {
                            timer_inner.is_running.store(true, Ordering::Relaxed)
                        }
                        TimerCommand::Pause => continue,
                        TimerCommand::End => break,
                    }
                }

                // increment timer
                let before_sleep = Instant::now();

                thread::sleep(Duration::from_millis(100));

                let time_slept = Instant::now() - before_sleep;

                let mut time_elapsed = timer_inner.time_elapsed.lock().unwrap();

                *time_elapsed += time_slept;
            }
        });

        Timer {
            command_tx,
            thread_handle,
            duration,
            time_elapsed,
            is_running,
        }
    }
}
