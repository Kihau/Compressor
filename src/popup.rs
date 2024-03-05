use std::time::Duration;

use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

#[allow(dead_code)]
pub struct MessageLog {
    pub text: &'static str,
    pub kind: ToastKind,
}

pub trait LogKind {
    fn success(&mut self, message: &'static str);
    fn error(&mut self, message: &'static str);
    fn warning(&mut self, message: &'static str);
    fn info(&mut self, message: &'static str);
}

fn default_options() -> ToastOptions {
    let duration = Duration::from_secs(3);
    let options = ToastOptions::default()
        .show_progress(true)
        .show_icon(true)
        .duration(Some(duration));

    return options;
}

impl LogKind for Toasts {
    fn success(&mut self, message: &'static str) {
        let log = Toast {
            text: message.into(),
            kind: ToastKind::Success,
            options: default_options(),
        };
        self.add(log);
    }

    fn error(&mut self, message: &'static str) {
        let log = Toast {
            text: message.into(),
            kind: ToastKind::Error,
            options: default_options(),
        };
        self.add(log);
    }

    fn warning(&mut self, message: &'static str) {
        let log = Toast {
            text: message.into(),
            kind: ToastKind::Warning,
            options: default_options(),
        };
        self.add(log);
    }

    fn info(&mut self, message: &'static str) {
        let log = Toast {
            text: message.into(),
            kind: ToastKind::Info,
            options: default_options(),
        };
        self.add(log);
    }
}

// struct Popup {
//     infinite_timeout: bool,
//     timeout_start: Option<Instant>,
//     message: String,
// }
//
// // Replace this with the notification crate???
// impl Popup {
//     fn new() -> Self {
//         Self {
//             // show: false,
//             // timeout_start: Instant::now(),
//             timeout_start: None,
//             infinite_timeout: true,
//             message: String::new(),
//         }
//     }
//
//     fn show_popup(&mut self, message: &str, infinite_timeout: bool) {
//         self.timeout_start = Some(Instant::now());
//         self.message = String::from(message);
//         self.infinite_timeout = infinite_timeout;
//     }
//
//     fn clear_popup(&mut self) {
//         self.timeout_start = None;
//         self.infinite_timeout = false;
//     }
//
//     fn update_popup(&mut self, ui: &mut egui::Ui) {
//         let now = Instant::now();
//         let Some(start) = self.timeout_start else {
//             return;
//         };
//
//         if now - start > TIMEOUT && !self.infinite_timeout {
//             self.clear_popup();
//         } else {
//             egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("popup"), |ui| {
//                 ui.label(&self.message);
//             });
//         }
//     }
// }
