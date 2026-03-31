use notify_rust::Notification;

use crate::pommo_core::PommoType;

fn notify_completed(notification_type: PommoType) {
    match notification_type {
        PommoType::Work => {
            send_notification("Pomodoro complete!", "Time for a break!");
        }
        PommoType::Break => {
            send_notification("Break complete!", "Time to get back to work!");
        }
    };
}

fn send_notification(summary: &str, body: &str) {
    Notification::new()
        .summary(summary)
        .body(body)
        .show()
        .expect("Failed to send notification");
}

#[derive(Debug, Default)]
pub struct NotificationManger {
    previous_notification_type: Option<PommoType>,
}

impl NotificationManger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify(&mut self, notification_type: PommoType) {
        if self.previous_notification_type == Some(notification_type) {
            return;
        }

        notify_completed(notification_type);

        self.previous_notification_type = Some(notification_type);
    }
}
