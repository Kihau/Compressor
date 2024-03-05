use std::fs::create_dir_all;

use eframe::egui::{self, Button};
use directories_next::ProjectDirs;

use crate::*;

pub struct DependencyDownloader {
    ffmpeg_missing: bool,
    ytdlp_missing: bool,
}

impl DependencyDownloader {
    pub fn new(_cc: &eframe::CreationContext<'_>, has_ffmpeg: bool, has_ytdlp: bool) -> Self {
        Self { ffmpeg_missing: has_ffmpeg, ytdlp_missing: has_ytdlp }
    }
}

impl eframe::App for DependencyDownloader {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.separator();
            ui.label("It looks like some of the dependencies are missing...");
            ui.separator();
            ui.label("You can quick-download them by clicking the buttons below:");
            ui.separator();
            ui.horizontal(|ui|{
                ui.label("yt-dlp:");

                let ytdlp_btn = ui.add_enabled(self.ytdlp_missing, Button::new("Download"));
                if ytdlp_btn.clicked() {
                    let output = download_ytdlp();
                    println!("{output:?}");
                    self.ytdlp_missing = false;
                }

                ui.add(egui::Separator::default().vertical());

                ui.label("ffmpeg:");
                let ffmpeg_btn = ui.add_enabled(self.ffmpeg_missing, Button::new("Download"));
                if ffmpeg_btn.clicked() {
                    let output = download_ffmpeg();
                    println!("{output:?}");
                    self.ffmpeg_missing = false;
                }
            });

            ui.separator();

            ui.label("You could also download them manually and place in tha same");
            ui.label("directory as compressor or add them as an enviroment variables.");

            ui.separator();

            ui.horizontal(|ui|{
                if ui.button("All Done, Let's Go!").clicked() {
                    frame.close()
                }

                ui.add(egui::Separator::default().vertical());

                if ui.button("Clear cache").clicked() {
                    let proj_dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).unwrap();
                    let cache_dir = proj_dirs.cache_dir();
                    let _ = std::fs::remove_dir_all(cache_dir);
                }

                ui.add(egui::Separator::default().vertical());

                if ui.button("Nope, I'm Out...").clicked() {
                    std::process::exit(0);
                }
            });

            ui.separator();
        });
    }
}

fn download_ffmpeg() -> Result<(), &'static str> {
    #[cfg(not(target_os = "windows"))]
    let download_dir = "ffmpeg-master-latest-linux64-lgpl";
    #[cfg(not(target_os = "windows"))]
    let download_string = &format!("{download_dir}.tar.xz");

    #[cfg(target_os = "windows")]
    let download_dir = "ffmpeg-master-latest-win64-gpl";
    #[cfg(target_os = "windows")]
    let download_string = &format!("{download_dir}.zip");

    let url = format!("https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/{download_string}");

    let Ok(mut response) = reqwest::blocking::get(url) else {
        return Err("Failed to download the ffmpeg zip archive");
    };

    let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) else {
        return Err("Failed to find the data directories");
    };

    let cache_dir = proj_dirs.cache_dir();
    if !cache_dir.exists() {
        let _ = create_dir_all(cache_dir);
    }

    let mut download_path = std::path::PathBuf::from(cache_dir);
    download_path.push(download_string);
    println!("{download_path:?}");

    let Ok(mut downloaded_achive) = File::create(&download_path) else {
        return Err("Failed to save downloaded ffmpeg zip archive");
    };

    if std::io::copy(&mut response, &mut downloaded_achive).is_err() {
        return Err("Failed to save downloaded ffmpeg zip archive");
    }

    // Unpacking the archives
    #[cfg(not(target_os = "windows"))]
    {
        let process = Command::new("tar")
            .args(["xf", &download_path.to_string_lossy()])
            .spawn();

        if let Ok(mut process) = process {
            let _ = process.wait();
        }
    }

    #[cfg(target_os = "windows")]
    {
        use zip::ZipArchive;

        println!("{download_path:?}");
        let reader = File::open(&download_path).unwrap();
        let Ok(mut zip_archive) = ZipArchive::new(reader) else {
            return Err("Failed to open the zip reader");
        };

        println!("{cache_dir:?}");
        // Extract to cache dir and then nuke it?
        if zip_archive.extract(cache_dir).is_err() {
            return Err("Failed to unpack downloaded zip");
        }
    }


    let data_dir = proj_dirs.data_dir();
    if !data_dir.exists() {
        let _ = create_dir_all(data_dir);
    }
    let output_path = std::path::PathBuf::from(data_dir);

    download_path.pop();
    download_path.push(download_dir);
    download_path.push("bin");
    println!("{download_path:?}");

    // let dir_path = format!("{download_dir}/bin");
    let directory = std::fs::read_dir(&download_path).unwrap(); 

    download_path.pop();
    for entry in directory {
        let filename = entry.unwrap().file_name();
        let mut old_path = download_path.clone();
        old_path.push("bin");
        old_path.push(&filename);

        let mut new_path = output_path.clone();
        new_path.push(&filename);
        
        println!("{old_path:?} -> {new_path:?}");
        let _ = std::fs::rename(old_path, new_path);
    }

    Ok(())   
}

fn download_ytdlp() -> Result<(), &'static str> {
    #[cfg(not(target_os = "windows"))]
    let ytdlp = "yt-dlp";
    #[cfg(target_os = "windows")]
    let ytdlp = "yt-dlp.exe";

    let url = format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{ytdlp}");

    let Ok(mut response) = reqwest::blocking::get(url) else {
        return Err("Failed to download yt-dlp executable");
    };

    let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) else {
        return Err("Failed to find the data directories");
    };

    let data_dir = proj_dirs.data_dir();
    if !data_dir.exists() {
        let _ = create_dir_all(data_dir);
    }

    let mut download_path = std::path::PathBuf::from(data_dir);
    download_path.push(ytdlp);
    // download_path.set_file_name(ytdlp);
    println!("{download_path:?}");

    let Ok(mut output_file) = File::create(&download_path) else {
        return Err("Failed to create yt-dlp executable file");
    };

    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let res = fs::set_permissions(&download_path, fs::Permissions::from_mode(0o755));
        if res.is_err() {
            return Err("File downloaded successfully. but failed to set 755 permissions")
        }
    }

    if std::io::copy(&mut response, &mut output_file).is_err() {
        return Err("Failed to copy bytes to created yt-dlp executable file");
    }

    Ok(())
}
