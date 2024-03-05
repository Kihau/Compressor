use std::{
    process::Command,
    env::current_dir, path::Path, fs::File
};

pub mod deps_download;
pub mod popup;
// pub mod compressor;
pub mod app;

pub const QUALIFIER: &str    = "net";
pub const ORGANIZATION: &str = "Kihau";
pub const APPLICATION: &str  = "Compressor";

pub const GUI_SCALE: f32 = 1.66;

pub fn get_media_info(media_path: &str) -> Option<String> {
    let Some(command) = get_command_string("ffmpeg") else {
        return None;
    };
    let media_info = Command::new(command)
        .args([ "-i", media_path ])
        .output();

    let Ok(media_info) = media_info else {
        return None;
    };

    Some(String::from_utf8_lossy(&media_info.stderr).to_string())
}

pub fn extract_media_duration(media_string: &str) -> Option<u64> {
    let Some(start_idx) = media_string.find("Duration:") else {
        return None;
    };

    let mut duration = 0u64;
    let Some(duration_string) = media_string[start_idx..].split(' ').nth(1) else {
        return None;
    };
    let chars: Vec<_> = duration_string.chars().collect();

    // Hours -> ms
    duration += chars[0].to_digit(10).unwrap() as u64 * 1000 * 3600 * 10;
    duration += chars[1].to_digit(10).unwrap() as u64 * 1000 * 3600;
    // Mintues -> ms
    duration += chars[3].to_digit(10).unwrap() as u64 * 1000 * 60 * 10;
    duration += chars[4].to_digit(10).unwrap() as u64 * 1000 * 60;
    // Seconds -> ms
    duration += chars[6].to_digit(10).unwrap() as u64 * 1000 * 10;
    duration += chars[7].to_digit(10).unwrap() as u64 * 1000;
    // Miliseconds
    duration += chars[9].to_digit(10).unwrap() as u64 * 10;
    duration += chars[10].to_digit(10).unwrap() as u64;

    Some(duration)
}

// pub fn get_ffmpeg_string() -> String {
//     #[cfg(not(target_os = "windows"))]
//     let ffmpeg = String::from("ffmpeg");
//     #[cfg(target_os = "windows")]
//     let ffmpeg = String::from("ffmpeg.exe");
//
//     return if Path::new(&ffmpeg).exists() {
//         let current = current_dir().unwrap();
//         let current = current.to_str().unwrap();
//         format!("{current}/{ffmpeg}")
//     } else {
//         // Assume that ffmpeg is in path
//         // Change it later?
//         ffmpeg
//     };
// }
//
// pub fn get_ytdlp_string() -> String {
//     #[cfg(not(target_os = "windows"))]
//     let ytdlp = String::from("yt-dlp");
//     #[cfg(target_os = "windows")]
//     let ytdlp = String::from("yt-dlp.exe");
//
//     match Command::new(&ytdlp).spawn() {
//         Ok(_) => return ytdlp,
//         Err(e) if e.kind() != std::io::ErrorKind::NotFound => return ytdlp,
//         _ => {}
//     };
//
//     if Path::new(&ytdlp).exists() {
//         let current = current_dir().unwrap();
//         let current = current.to_str().unwrap();
//         return format!("{current}/{ytdlp}");
//     }
//
//     ytdlp
// }

// Checking priority:
//     path -> AppData and XDG_DATA -> current dir
pub fn get_command_string(cmd_str: &str) -> Option<String> {
    #[cfg(not(target_os = "windows"))]
    let command = String::from(cmd_str);
    #[cfg(target_os = "windows")]
    let command = format!("{cmd_str}.exe");

    // Check if command is in path
    let mut env_check = Command::new(&command);
    env_check.stdout(std::process::Stdio::null());
    env_check.stderr(std::process::Stdio::null());
    match env_check.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
            return Some(command);
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Some(command),
        _ => {}
    };

    use directories_next::ProjectDirs;
    if let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) {
        let data_dir = proj_dirs.data_dir();

        let mut cmd_buf = std::path::PathBuf::from(data_dir);
        cmd_buf.push(&command);
        // cmd_buf.set_file_name(&command);

        println!("{cmd_buf:?}");
        if cmd_buf.exists() {
            return Some(cmd_buf.to_string_lossy().to_string());
        }
    }

    if Path::new(&command).exists() {
        let Ok(current) = current_dir() else { return None };
        let Some(current) = current.to_str() else { return None };
        return Some(format!("{current}/{command}"));
    }

    None
}
