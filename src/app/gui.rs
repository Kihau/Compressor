use std::{path::{Path, PathBuf}, time::Duration};
use eframe::egui::*;
use egui_toast::{Toast, ToastOptions};

use crate::{*, popup::LogKind};
use super::*;

impl eframe::App for Compressor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        ctx.set_pixels_per_point(GUI_SCALE);
        self.popup.show(ctx);

        self.receive_log_messages();

        if let Some(last_update_time) = self.last_state_update {
            let time_now = Instant::now();
            if time_now - last_update_time >= Duration::from_secs(1) {
                self.save_cache();
                self.last_state_update = None;
            }
        }

        // let scale = ctx.pixels_per_point();
        // println!("pixels per point: {scale}");
        
        // let winfo = _frame.info().window_info;
        // println!("DEBUGGING: {winfo:?}");

        if let Some(rx) = &self.work_finished_rx {
            if let Ok(()) = rx.try_recv() {
                self.is_working = false;
            }
        }

        if self.should_exit && !self.is_working {
            self.save_cache();

            println!("Good bye!");
            std::process::exit(0);
        }

        match &self.work_progress_rx {
            Some(work_progress_rx) if self.is_working => {
                if let Ok(new_progress) = work_progress_rx.try_recv() {
                    self.progress = new_progress;
                }
            }
            _ => {}
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.add_top_options(ui);
            ui.separator();

            if matches!(self.selected_mode, AppMode::Advanced) {
                self.add_advanced_settings(ui);
            } else {
                self.add_quality_settings(ui);
            }
            ui.separator();

            self.add_custom_size_picker(ui);
            ui.separator();

            self.add_input_selectors(ui);
            ui.separator();

            self.add_bottom_buttons(ui);

            ui.add_space(10.0);

            if !self.is_working {
                ui.label("Waiting for a convertion/download job...");
            } else if matches!(self.selected_mode, AppMode::Download) {
                ui.label(format!("Downloading in progress: [{:.2}%]", (self.progress * 100.0)));
            } else {
                ui.label(format!("Convertion in progress: [{:.2}%]", (self.progress * 100.0)));
            };

            ui.add_enabled(
                self.is_working, egui::ProgressBar::new(self.progress).animate(self.is_working)
            );
        });
    }
}

impl Compressor {
    pub(super) fn try_output_from_input_path(&mut self) -> Option<String> {
        let extension = match self.selected_mode {
            AppMode::Audio    => "mp3",
            AppMode::Video    => "mp4",
            AppMode::Download => return None,
            AppMode::Advanced => return None,
            // AppMode::DualMode => return None,
        };

        let input_path = Path::new(&self.input_path);
        if !input_path.exists() || input_path.is_dir() {
            return None;
        }

        let Some(input_filename) = input_path.file_stem() else {
            return None;
        };

        let input_filename = input_filename.to_str().unwrap();
        let mut output = PathBuf::from(input_path.parent().unwrap());
        output.push(format!("{input_filename}-new.{extension}"));

        Some(String::from(output.to_string_lossy()))
    }

    fn add_custom_size_picker(&mut self, ui: &mut egui::Ui) {
        let btn = ui.add_enabled(
            !self.is_working && !matches!(self.selected_mode, AppMode::Download),
            RadioButton::new(
                self.use_output_file_size, 
                "Approximate size of the output file (automatic quality selection):"
            )
        );

        if btn.clicked() {
            self.use_output_file_size = !self.use_output_file_size;
        }

        ui.horizontal(|ui| {

            ui.add_enabled_ui(
                self.use_output_file_size && !matches!(self.selected_mode, AppMode::Download) && 
                !self.is_working, |ui| {
                ui.label("Video / Audio bitrate ratio:");
                ui.add(Slider::new(&mut self.bitrate_ratio, 0.0..=20.0).text("/ 1.0"));
                ui.add(egui::Separator::default().vertical());
                ui.label("Size in MB:");
                ui.add(TextEdit::singleline(&mut self.output_file_size).desired_width(f32::INFINITY));
            });
        });

    }

    fn add_top_options(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let audio_btn = RadioButton::new(matches!(self.selected_mode, AppMode::Audio), "Audio");
            if ui.add_enabled(!self.is_working, audio_btn).clicked() {
                self.selected_mode = AppMode::Audio;
            }

            let video_btn = RadioButton::new(matches!(self.selected_mode, AppMode::Video), "Video");
            if ui.add_enabled(!self.is_working, video_btn).clicked() {
                self.selected_mode = AppMode::Video;
            }

            let download_btn = RadioButton::new(matches!(self.selected_mode, AppMode::Download) || self.dual_mode, "Downloading");
            if ui.add_enabled(!self.is_working && !self.dual_mode, download_btn).clicked() {
                self.selected_mode = AppMode::Download;
            }

            let advanced_btn = RadioButton::new(matches!(self.selected_mode, AppMode::Advanced), "Advanced");
            if ui.add_enabled(!self.is_working && !self.is_working, advanced_btn).clicked() {
                self.selected_mode = AppMode::Advanced;
            }

            let dual_btn = RadioButton::new(self.dual_mode, "Dual Mode");
            if ui.add_enabled(!self.is_working, dual_btn).clicked() {
                if !self.dual_mode && matches!(self.selected_mode, AppMode::Download) {
                    self.selected_mode = AppMode::Audio;
                }
                self.dual_mode = !self.dual_mode;
            }

        });
    }

    fn add_advanced_settings(&mut self, _ui: &mut egui::Ui) {
        // TODO
    }

    fn add_quality_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Audio quality: ");

                for (i, text) in QUALITY_GUI_LABELS.iter().enumerate() {
                    let resp = ui.add_enabled(
                        !self.is_working && !matches!(self.selected_mode, AppMode::Download) && !self.use_output_file_size, 
                        RadioButton::new(i == self.audio_quality as usize, *text)
                    );

                    if resp.clicked() {
                        self.audio_quality = Quality::from_usize(i);
                    }
                }
            });

            ui.add(egui::Separator::default().vertical());

            ui.vertical(|ui| {
                ui.label("Video quality: ");

                for (i, text) in QUALITY_GUI_LABELS.iter().enumerate() {
                    let resp = ui.add_enabled(
                        !self.is_working && matches!(self.selected_mode, AppMode::Video) && !self.use_output_file_size,
                        RadioButton::new(i == self.video_quality as usize, *text)
                    );

                    if resp.clicked() {
                        self.video_quality = Quality::from_usize(i);
                    }
                }
            });

            ui.add(egui::Separator::default().vertical());

            ui.vertical(|ui| {
                let resolution_btn = ui.add_enabled(
                    !self.is_working && matches!(self.selected_mode, AppMode::Video),
                    RadioButton::new(self.use_custom_resolution, "Resolution:")
                );

                if resolution_btn.clicked() {
                    self.use_custom_resolution = !self.use_custom_resolution;
                }

                for (i, text) in RESOLUTION_GUI_LABELS.iter().enumerate() {
                    let resp = ui.add_enabled(
                        !self.is_working && matches!(self.selected_mode, AppMode::Video) && self.use_custom_resolution,
                        RadioButton::new(i == self.selected_resolution, *text)
                    );

                    if resp.clicked() {
                        self.selected_resolution = i;
                    }
                }

            });

            ui.add(egui::Separator::default().vertical());

            ui.vertical(|ui| {
                ui.label("Compression:");

                for (i, text) in PRESET_GUI_LABELS.iter().enumerate() {
                    let resp = ui.add_enabled(
                        !self.is_working && matches!(self.selected_mode, AppMode::Video),
                        RadioButton::new(i == self.selected_preset, *text)
                    );

                    if resp.clicked() {
                        self.selected_preset = i;
                    }
                }
            });
        });
    }

    fn add_input_selectors(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let inputline_string = if matches!(self.selected_mode, AppMode::Download) {
                "Link to a media you wish to download:"
            } else {
                "Path to a file you wish to convert:"
            };
            ui.label(inputline_string);

            let pick_file_btn = ui.add_enabled(
                !self.is_working && !matches!(self.selected_mode, AppMode::Download),
                Button::new("File selection")
            );

            if pick_file_btn.clicked() {
                let res = rfd::FileDialog::new()
                    .pick_file();

                if let Some(path) = res {
                    self.input_path = path.to_str().unwrap().to_string();
                    self.save_cache();
                }
            }
        });


        ui.horizontal(|ui| {
            if ui.add_enabled(!self.is_working, Button::new("🗑")).clicked() {
                self.input_path.clear();
                self.save_cache();
            }

            let input_path_field = ui.add_enabled(
                !self.is_working,
                TextEdit::singleline(&mut self.input_path).desired_width(f32::INFINITY)
            );

            if input_path_field.changed() {
                self.last_state_update = Some(Instant::now());
            }
        });

        ui.horizontal(|ui| {
            ui.label("Output file (can be empty):");

            if ui.add_enabled(!self.is_working, Button::new("File saving")).clicked() {
                let res = rfd::FileDialog::new()
                    .save_file();

                if let Some(path) = res {
                    self.output_path = path.to_str().unwrap().to_string();
                    self.save_cache();
                }
            }
        });

        ui.horizontal(|ui| {
            if ui.add_enabled(!self.is_working, Button::new("🗑")).clicked() {
                self.output_path.clear();
                self.save_cache();
            }

            let output_hinting = if let Some(auto_output) = self.try_output_from_input_path() {
                auto_output
            } else {
                String::new()
            };

            let output_textbox = TextEdit::singleline(&mut self.output_path)
                .desired_width(f32::INFINITY).hint_text(output_hinting);

            // let output_textbox = TextEdit::singleline(&mut self.output_path).desired_width(f32::INFINITY);
            let output_path_field = ui.add_enabled(!self.is_working, output_textbox);
            if output_path_field.changed() {
                self.last_state_update = Some(Instant::now());
            }
        });
    }

    fn create_new_channels(&mut self) -> ThreadChannels {
        let (log_tx, log_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished_rx) = std::sync::mpsc::channel();
        let (abort_tx, abort_rx)       = std::sync::mpsc::channel();
        let (progress_tx, progress_rx) = std::sync::mpsc::channel();

        self.message_log_rx   = Some(log_rx);
        self.work_finished_rx = Some(finished_rx);
        self.work_abort_tx    = Some(abort_tx);
        self.work_progress_rx = Some(progress_rx);

        let channels = ThreadChannels::new(log_tx, finished_tx, abort_rx, progress_tx);
        return channels;
    }

    fn add_bottom_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let action_btn = if self.is_working {
                "Cancel"
            } else if matches!(self.selected_mode, AppMode::Download) {
                "Download"
            } else {
                "Convert"
            };

            if ui.button(action_btn).clicked() {
                if self.is_working {
                    if let Some(tx) = &self.work_abort_tx {
                        let _ = tx.send(());
                    }
                    return;
                } 

                if self.input_path.trim().is_empty() {
                    if matches!(self.selected_mode, AppMode::Download) {
                        self.popup.error("Input cannot be empty. You must provide a download link.");
                    } else {
                        self.popup.error("Input cannot be empty. You must provide a path to a file.");
                    }
                    return;
                }

                if !matches!(self.selected_mode, AppMode::Download) && !Path::new(&self.input_path).exists() {
                    self.popup.error("Input path is incorrect, file does not exist.");
                } else {
                    self.is_working = true;
                    self.progress = 0.0;

                    let channels = self.create_new_channels();
                    match self.selected_mode {
                        AppMode::Audio    => self.compress_audio(channels),
                        AppMode::Video    => self.compress_video(channels),
                        AppMode::Download => self.download_resource(channels),
                        AppMode::Advanced => self.compress_advanced(channels),
                    }
                }
            }

            if ui.button("Exit").clicked() {
                if let Some(tx) = &self.work_abort_tx {
                    let _ = tx.send(());
                }

                self.should_exit = true;
            }

            let reset_btn = ui.add_enabled(!self.is_working, Button::new("Reset"));
            if reset_btn.clicked() {
                *self = Default::default();
                self.popup.info("App was reset to its initial state");
                self.clear_cache();
            }

            #[cfg(not(target_os = "windows"))]
            let _ = ui.add_enabled(false, Button::new("Hide console"));

            if ui.button("Clear popups").clicked() {
                // Just grab the entire source code of the "library"
                // It literally is just two file, no need to use cargo
                todo!()
            }
        });
    }

    fn receive_log_messages(&mut self) {
        let Some(log_rx) = &self.message_log_rx else {
            return;
        };

        let Ok(log) = log_rx.try_recv() else {
            return;
        };

        let toast = Toast {
            text: log.text.into(),
            kind: log.kind,
            options: ToastOptions::default(),
        };

        self.popup.add(toast);

        // match message {
        //     MessageLog::Success(s) => self.popup.success(s),
        //     MessageLog::Basic(s)   => self.popup.basic(s),
        //     MessageLog::Info(s)    => self.popup.info(s),
        //     MessageLog::Warning(s) => self.popup.warning(s),
        //     MessageLog::Error(s)   => self.popup.error(s),
        // };
    }
}
