use std::{
    sync::mpsc::{Receiver, Sender}, path::PathBuf, fs, time::Instant, io::Write, 
};

use eframe::{emath::Align2, egui};
use egui_toast::{Toasts, ToastKind};

use crate::{*, popup::MessageLog};

mod gui;
mod processes;

// use gui::*;
// use processes::*;

const PRESET_FFMPEG_STRINGS:     &[&str] = &[ "veryslow", "slow", "medium", "fast", "ultrafast", ];
const RESOLUTION_FFMPEG_STRINGS: &[&str] = &[ "1080", "720", "480", "360", "144", ];
const PRESET_GUI_LABELS:         &[&str] = &[ "Very Precise", "Precise", "Balanced", "Fast", "Very Fast", ];
const RESOLUTION_GUI_LABELS:     &[&str] = &[ "1080p", "720p", "480p", "360p", "144p", ];
const QUALITY_GUI_LABELS:        &[&str] = &[ "Original", "Good", "Medium", "Bad", "Poop" ];

// Qulity could be a fraction of the original bitrate 
#[derive(Default, Copy, Clone)]
enum Quality {
    #[default]
    // 100%
    Original = 0,
    // 90%
    Good     = 1,
    // 60%
    Medium   = 2,
    // 30%
    Bad      = 3,
    // 10%
    Poop     = 4,
}

#[allow(dead_code)]
#[derive(Default)]
enum AppMode {
    #[default]
    Audio    = 0,
    Video    = 1,
    // Audio and Video quality selection option for download (or maybe don't to this and just reencode?)
    Download = 2,
    // When advanced is selected you can still select either audio or video but the selections the
    // quality options are greatly expanded (audio bitrade/sample rate, video bitrate (actual kB/s values in the selections))
    Advanced = 3,
    // Download mode is always selected, but you can also select either audio or video option
    // DualMode = 4,
}

#[allow(dead_code)]
enum OldMessageLog {
    Basic(&'static str),
    Info(&'static str),
    Success(&'static str),
    Warning(&'static str),
    Error(&'static str),
}

struct ThreadChannels {
    message_log_tx: Sender<MessageLog>,
    work_finished_tx: Sender<()>,
    work_abort_rx: Receiver<()>,
    work_progress_tx: Sender<f32>,
}

#[allow(dead_code)]
struct ConvertOptions {
    audio_enabled: bool,
    audio_bitrate: u32,
    audio_sample_rate: u32,

    video_enabled: bool,
    video_bitrate: u32,
    video_width: u32,
    video_height: u32,

    preset: &'static str,
    tune: &'static str,
    crf: u8,
}


impl Quality {
    fn from_usize(num: usize) -> Self {
        match num {
            0 => Quality::Original,
            1 => Quality::Good,
            2 => Quality::Medium,
            3 => Quality::Bad,
            4 => Quality::Poop,
            _ => panic!("Incorrect size"),
        }
    }
}

//
// TODO: Create job queue, allow user (me) to queue multiple convertion jobs (that would also mean
//       that you could queue multiple files at once, the ui should also display the queue)
//
//       Job Queue: (an example of running download and two-pass convertion job)
//       +-------+----------+----------+------------------+---------------+-------------------+
//       | Btns  | Position | Job name | The input        | The Output    | Progress / Status |
//       +-------+----------+----------+------------------+---------------+-------------------+
//       | T ^ v |    1.    | Download | https://link.com | something.mp4 | [###-------] 30 % |
//       | T ^ v |    2.    | Analize  | something.mp4    |      -        | [----------]  0 % |
//       | T ^ v |    3.    | Convert  | something.mp4    | output.mp4    | [----------]  0 % |
//       +-------+----------+----------+------------------+---------------+-------------------+
//
//       - The T icon is going to be the "Trash" emote - remove from queue button
//       - The "^" emote is move up and "v" emote is move down
//

struct DownloadJob {
    input_link: String,
    output_file: String
}

struct ConvertionJob {
    input_file: String,
    output_file: String
    // Other options
}

enum CompressorJob {
    Convert(Box<ConvertionJob>),
    Download(Box<DownloadJob>),
    Analyze,
}

/// Represents the state of the GUI and holds the data of the program
pub struct Compressor {
    //
    // State of the program
    //

    is_working: bool,
    should_exit: bool,
    progress: f32,

    /// Timer storing the last gui state update time. Set to when cache is up to date.
    last_state_update: Option<Instant>,

    //
    // Communication between threads
    //
    
    /// Receives a message from the working thread that gets displayed by the popup
    message_log_rx: Option<Receiver<MessageLog>>,
    /// Receives an information from the working thread that the work has finished
    work_finished_rx: Option<Receiver<()>>,
    /// Sends an information to work threads to abort its execution
    work_abort_tx: Option<Sender<()>>,
    // Sends current progress status to the GUI thread (value between 0.0 and 1.0)
    work_progress_rx: Option<Receiver<f32>>,

    //
    // Options selected in the main gui thread
    //
    
    /// The input textbox buffer
    input_path: String,
    /// The output textbox buffer
    output_path: String,

    selected_mode: AppMode,
    // TODO: Make this an optional - user can disable the audio if in AppMode::Video or Advanced
    audio_quality: Quality,
    video_quality: Quality,

    dual_mode: bool,
    // advanced_mode: bool,

    /// Output resolution of the video
    use_custom_resolution: bool,
    selected_resolution: usize,

    /// The -preset flag used by ffmpeg (default preset is always medium).
    selected_preset: usize,
    
    /// Approximate output file size in MB
    use_output_file_size: bool,
    output_file_size: String,
    bitrate_ratio: f32,

    /// The popups that show up in the top right corner
    popup: Toasts,
}

impl Compressor {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    pub fn load_cache_if_present(&mut self)  {
        use directories_next::ProjectDirs;

        let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) else {
            println!("INFO: Couldn't create/get cache directory?");
            return
        };

        let cache_dir = proj_dirs.cache_dir();
        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("app_cache.txt");

        let Ok(cache) = fs::read_to_string(cache_path) else {
            println!("INFO: Couldn't load cache, perhaps the cache file is not present");
            return
        };

        if let Some(line) = cache.lines().nth(0) {
            self.input_path = String::from(line.trim());
        }

        if let Some(line) = cache.lines().nth(1) {
            self.output_path = String::from(line.trim());
        }

        println!("INFO: Loaded the cache (maybe)");
    }

    pub fn clear_cache(&self)  {
        use directories_next::ProjectDirs;

        let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) else {
            println!("INFO: Couldn't create/get cache directory?");
            return
        };

        let cache_dir = proj_dirs.cache_dir();
        if !cache_dir.exists() {
            let _ = fs::create_dir_all(cache_dir);
        }

        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("app_cache.txt");

        let Ok(cache_file) = fs::File::create(cache_path) else {
            println!("ERROR: Failed to create/truncate cache file. Fix ya filesystem...");
            return;
        };

        let _ = cache_file.set_len(0);
        println!("INFO: Cleared cache?");
    }

    pub fn save_cache(&self)  {
        use directories_next::ProjectDirs;

        let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) else {
            println!("INFO: Couldn't create/get cache directory?");
            return
        };

        let cache_dir = proj_dirs.cache_dir();
        if !cache_dir.exists() {
            let _ = fs::create_dir_all(cache_dir);
        }

        let mut cache_path = PathBuf::from(cache_dir);
        cache_path.push("app_cache.txt");

        let Ok(mut cache_file) = fs::File::create(cache_path) else {
            println!("ERROR: Failed to create/truncate cache file. Fix ya filesystem...");
            return;
        };

        let _ = cache_file.write_fmt(format_args!("{}\n{}\n", self.input_path, self.output_path));
        println!("INFO: Saved the cache! (maybe)");
    }
}

#[allow(dead_code)]
impl ThreadChannels {
    fn new(log: Sender<MessageLog>, finished: Sender<()>, abort: Receiver<()>, progress: Sender<f32>) -> Self {
        Self {
            message_log_tx: log,
            work_finished_tx: finished,
            work_abort_rx: abort,
            work_progress_tx: progress,
        }
    }

    fn send_error(&self, message: &'static str) {
        let log = MessageLog {
            text: message,
            kind: ToastKind::Error,
        };
        let _ = self.message_log_tx.send(log);
        let _ = self.work_finished_tx.send(());
    }

    fn send_info(&self, message: &'static str) {
        let log = MessageLog {
            text: message,
            kind: ToastKind::Info,
        };
        let _ = self.message_log_tx.send(log);
        let _ = self.work_finished_tx.send(());
    }

    fn send_success(&self, message: &'static str) {
        let log = MessageLog {
            text: message,
            kind: ToastKind::Success,
        };
        let _ = self.message_log_tx.send(log);
        let _ = self.work_finished_tx.send(());
    }

    fn send_warning(&self, message: &'static str) {
        let log = MessageLog {
            text: message,
            kind: ToastKind::Warning,
        };
        let _ = self.message_log_tx.send(log);
        let _ = self.work_finished_tx.send(());
    }

    fn send_custom(&self, message: &'static str, custom: u32) {
        let log = MessageLog {
            text: message,
            kind: ToastKind::Custom(custom),
        };
        let _ = self.message_log_tx.send(log);
        let _ = self.work_finished_tx.send(());
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            is_working: false,
            should_exit: false,
            progress: 1.0,
            last_state_update: None,

            message_log_rx: None,
            work_finished_rx: None,
            work_abort_tx: None,
            work_progress_rx: None,

            input_path: String::new(),
            output_path: String::new(),

            selected_mode: AppMode::default(),
            audio_quality: Quality::default(),
            video_quality: Quality::default(),

            dual_mode: false,
            // advanced_mode: false,

            use_custom_resolution: false,
            selected_resolution: 0,

            selected_preset: 2,

            use_output_file_size: false,
            output_file_size: String::from("0"),
            bitrate_ratio: 10.0,
        
            popup: Toasts::new()
                .anchor(Align2::RIGHT_TOP, (-10.0, 10.0))
                .direction(egui::Direction::TopDown),
        }
    }
}
