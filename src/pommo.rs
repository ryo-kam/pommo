// use std::{
//     sync::{
//         Arc, Mutex,
//         atomic::{AtomicBool, Ordering},
//     },
//     time::Duration,
// };

// const SHORT_BREAK: Pommo = Pommo {
//     pommo_type: PommoType::Break,
//     duration: Duration::from_mins(10),
// };

// const LONG_BREAK: Pommo = Pommo {
//     pommo_type: PommoType::Break,
//     duration: Duration::from_mins(10),
// };

// const WORK: Pommo = Pommo {
//     pommo_type: PommoType::Work,
//     duration: Duration::from_mins(10),
// };

// const POMMOS: &[Pommo] = &[
//     WORK,
//     SHORT_BREAK,
//     WORK,
//     SHORT_BREAK,
//     WORK,
//     SHORT_BREAK,
//     WORK,
//     LONG_BREAK,
// ];

// #[derive(Debug)]
// pub struct App {
//     current_pommo_index: usize,

//     is_ticking: AtomicBool,
//     time_elapsed: Arc<Mutex<Duration>>,
// }

// #[derive(Debug, Clone)]
// pub struct Pommo {
//     duration: Duration,
//     pommo_type: PommoType,
// }

// #[derive(Debug, Clone)]
// pub enum PommoType {
//     Break,
//     Work,
// }

// impl Default for App {
//     fn default() -> Self {
//         Self {
//             current_pommo_index: 0,
//             is_ticking: AtomicBool::new(false),
//             time_elapsed: Arc::new(Mutex::new(Duration::ZERO)),
//         }
//     }
// }

// impl App {
//     fn current_pommo(&self) -> &'static Pommo {
//         &POMMOS[self.current_pommo_index]
//     }

//     pub fn start_timer(&mut self) {
//         self.is_ticking.store(true, Ordering::Relaxed);

//         let mut time_elapsed_ref = self.time_elapsed.clone();

//         // thread::spawn(|| {
//         //     let mut wake_up_time = Instant::now();

//         //     while self.is_ticking.load(Ordering::Relaxed) {
//         //         wake_up_time += Duration::from_secs(1);

//         //         thread::sleep(wake_up_time - Instant::now());

//         //         let mut time_elapsed = time_elapsed_ref.lock().unwrap();
//         //         *time_elapsed += Duration::from_secs(1);

//         //         if self.time_elapsed >= self.current_pommo().duration {
//         //             self.is_ticking = false;
//         //             break;
//         //         }
//         //     }
//         // });
//     }
// }
