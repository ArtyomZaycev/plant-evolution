use std::{
    sync::LazyLock,
    time::{Duration, SystemTime},
};

use egui::{Align2, Label, Shadow, Vec2, Widget, mutex::Mutex};

pub struct Toast {
    closed: bool,

    created: SystemTime,
    text: String,
    autoclose: bool,
}

impl Toast {
    pub fn new(text: &str) -> Self {
        Self {
            closed: false,
            created: SystemTime::now(),
            text: text.to_owned(),
            autoclose: false,
        }
    }

    pub fn autoclose(self, autoclose: bool) -> Self {
        Self { autoclose, ..self }
    }
}

impl Widget for &Toast {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(Label::new(&self.text).wrap())
    }
}

pub struct ToastManager {
    ui_limit: u8,
    autoclose_time: Duration,
    toasts: Vec<Toast>,
    width: f32,
    border: f32,
}

impl ToastManager {
    fn new() -> Self {
        Self {
            ui_limit: 4,
            autoclose_time: Duration::from_secs(4),
            toasts: vec![],
            width: 320.,
            border: 8.,
        }
    }

    pub fn add(&mut self, notification: Toast) {
        self.toasts.push(notification);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let mut pos = ui.content_rect().left_bottom() + Vec2::new(self.border, -self.border);
        self.toasts
            .iter_mut()
            .enumerate()
            .for_each(|(i, notification)| {
                if notification.autoclose
                    && SystemTime::now()
                        .duration_since(notification.created)
                        .unwrap()
                        >= self.autoclose_time
                {
                    notification.closed = true;
                }
                if i < self.ui_limit as usize {
                    // TODO: it shouldn't be done like that
                    ui.style_mut().visuals.window_shadow = Shadow::NONE;
                    ui.style_mut().visuals.popup_shadow = Shadow::NONE;
                    ui.ctx().set_visuals(ui.style().visuals.clone());

                    let res = egui::Window::new(format!("toast_{i}"))
                        .max_width(self.width)
                        .min_width(self.width)
                        .default_width(self.width)
                        .fixed_pos(pos)
                        .pivot(Align2::LEFT_BOTTOM)
                        .resizable(false)
                        .title_bar(false)
                        .show(ui.ctx(), |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                if ui.button("X").clicked() {
                                    notification.closed = true;
                                }
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::TOP),
                                    |ui| {
                                        notification.ui(ui);
                                    },
                                );
                            });
                        });
                    if let Some(res) = res {
                        pos = res.response.rect.left_top();
                        pos.y -= self.border;
                    }
                }
            });

        self.toasts.retain(|n| !n.closed);
    }
}

pub static TOAST_MANAGER: LazyLock<Mutex<ToastManager>> =
    LazyLock::new(|| Mutex::new(ToastManager::new()));
