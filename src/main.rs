// Hide windows console for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] 

use compressor::{get_command_string, deps_download, GUI_SCALE, app::Compressor};
use eframe::egui;


// TODO: When only directory was provided as an input, convert all files in the directory ???
// (or maybe just add a radio button "Many")

// TODO: Also allow to run with cli options

fn main() -> Result<(), eframe::Error> {

    { // Check whether the required external dependencies are present
        let ffmpeg_missing = get_command_string("ffmpeg").is_none();
        let ytdlp_missing = get_command_string("yt-dlp").is_none();

        if ffmpeg_missing || ytdlp_missing {
            let _ = eframe::run_native(
                "Some programs are missing", 
                eframe::NativeOptions { 
                    default_theme: eframe::Theme::Dark,
                    follow_system_theme: false,
                    ..Default::default()
                },
                Box::new(move |cc| 
                    Box::new(deps_download::DependencyDownloader::new(cc, ffmpeg_missing, ytdlp_missing))
                ),
            );
        }
    }

    let options = eframe::NativeOptions {
        #[cfg(target_os = "windows")]
        initial_window_size: Some(egui::Vec2::new(500.0 * GUI_SCALE, 400.0 * GUI_SCALE)),
        #[cfg(not(target_os = "windows"))]
        initial_window_size: Some(egui::Vec2::new(300.0 * GUI_SCALE, 240.0 * GUI_SCALE)),

        resizable: true,
        default_theme: eframe::Theme::Dark, 
        follow_system_theme: false,
        ..Default::default()
    };

    eframe::run_native(
        "Compressor - ffmpeg and yt-dlp GUI", options, 
        Box::new(|cc| {
            let mut compressor = Compressor::new(cc);
            compressor.load_cache_if_present();
            Box::new(compressor) 
        }),
    )
}
