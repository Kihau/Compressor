use std::{
    process::Command, path::PathBuf, 
};

use super::*;

impl Compressor {
    fn get_output_path(&mut self) -> Option<String> {
        if self.output_path.trim().is_empty() {
            return self.try_output_from_input_path();
        } else {
            let mut output_buf = PathBuf::from(&self.output_path);
            if output_buf.extension().is_none() {
                match self.selected_mode {
                    AppMode::Audio => { output_buf.set_extension("mp3"); }
                    AppMode::Video => { output_buf.set_extension("mp4"); }
                    _ => {},
                }
            }
            return Some(String::from(output_buf.to_string_lossy()));
        };
    }

    pub(super) fn download_resource(&mut self, channels: ThreadChannels) {
        let Some(ytdlp) = get_command_string("yt-dlp") else {
            channels.send_error("The downloader could not be found. Please re-run the app to download required dependencies");
            return;
        };

        let mut process = Command::new(ytdlp);
        process.args([ "--progress", "--quiet", "--newline" ]);

        if !self.output_path.trim().is_empty() {
            process.args([ "-o", &self.output_path ]);
        }
        
        process.arg(&self.input_path);
        process.stdout(std::process::Stdio::piped());

        let Ok(mut process) = process.spawn() else {
            channels.send_error("Failed to start the downloading");
            return;
        };

        std::thread::spawn(move || {
            let status = loop { 
                if let Ok(()) = channels.work_abort_rx.try_recv() {
                    let _ = process.kill();
                    channels.send_info("Downloading process was canceled");
                    return;
                }

                let Ok(wait_result) = process.try_wait() else {
                    channels.send_error("Downloading unexpectedly stopped");
                    return;
                };

                if let Some(status) = wait_result {
                    break status;
                }

                let Some(stdout) = process.stdout.as_mut() else {
                    continue;
                };

                let mut byte_buffer = [0u8; 1024];

                use std::io::Read;
                let _bytes_read = stdout.read(&mut byte_buffer);

                let string_output = String::from_utf8_lossy(&byte_buffer);
                let Some(progress_string) = string_output.get(10..16) else {
                    continue;
                };
                let Ok(mut progress) = progress_string.trim().parse::<f32>() else {
                    continue;
                };

                progress /= 100.0;
                let _ = channels.work_progress_tx.send(progress);
            };

            if status.success() {
                channels.send_success("Download successful");
            } else {
                channels.send_error("Failed to download the file");
            }
        });
    }

    pub(super) fn compress_advanced(&mut self, _channels: ThreadChannels) {
        todo!()
    }

    pub(super) fn compress_video(&mut self, channels: ThreadChannels) {
        let Some(ffmpeg) = get_command_string("ffmpeg") else {
            channels.send_error("FFMPEG could not be found. Please re-run the app to download required dependencies");
            return;
        };
        let mut process = Command::new(&ffmpeg);

        // Setting up the ffmpeg process
        process.args([
            "-progress", "pipe:1",
            "-i", &self.input_path,
            "-y",
        ]);

        // Setting a custom resolution
        if self.use_custom_resolution {
            // ffmpeg -i input.mp4 -vf scale=-1:720,setdar=1:1 output.mp4
            let selected_resolution = RESOLUTION_FFMPEG_STRINGS[self.selected_resolution];
            let scale = format!("scale=-1:{selected_resolution}");
            process.args(&["-vf", &scale]);
        }

        // Setting a custom preset
        // ffmpeg -i input.mp4 -preset fast output.mp4
        let selected_preset = PRESET_FFMPEG_STRINGS[self.selected_preset]; 
        process.args(["-preset", selected_preset]);

        let Some(output_path) = self.get_output_path() else {
            channels.send_error("Failed to construct output path for selected input.");
            return;
        };
        process.arg(&output_path);

        if self.use_output_file_size {
            let Ok(expected_size) = self.output_file_size.parse::<f32>() else {
                channels.send_error("Failed to read the output file size.");
                return;
            };

            const CONVERTION_RATE: f32 = (1024.0 * 1024.0) / (1000.0 * 1000.0);
            let size_in_mib = expected_size * CONVERTION_RATE;

            let Some(media_string) = get_media_info(&self.input_path) else { 
                channels.send_error("Failed to retreive media info about thea provided file.");
                return;
            }; 

            let Some(media_duration) = extract_media_duration(&media_string) else {
                channels.send_error("Failed to retreive media length from the provided file.");
                return;
            }; 
            let duration_in_secs = media_duration as f32 / 1000.0;

            let total_bitrate = (size_in_mib * 8388.608) / duration_in_secs;
            let audio_bitrate = total_bitrate / 10.0;
            let video_bitrate = total_bitrate - audio_bitrate;

            //ffmpeg -y -i input -c:v libx264 -b:v 2600k -pass 1 -an -f null /dev/null && \
            //ffmpeg -i input -c:v libx264 -b:v 2600k -pass 2 -c:a aac -b:a 128k output.mp4
            //
            // Warning: When using option -an, you may eventually get a segfault or a broken file.
            //          If so, remove option -an and replace by -vsync cfr to the first pass.

            let mut first_pass = Command::new(&ffmpeg);
            let mut second_pass = process;

            first_pass.args(second_pass.get_args());

            first_pass.args([ "-c:v", "libx264" ]);
            first_pass.args([ "-b:v", &format!("{}K", video_bitrate as u32) ]);
            // first_pass.args([ "-pass", "1", "-vsync", "cfr", "-f", "null", "/dev/null" ]);
            first_pass.args([ "-pass", "1", "-an", "-f", "null", "/dev/null" ]);

            second_pass.args([ "-c:v", "libx264" ]);
            second_pass.args([ "-b:v", &format!("{}K", video_bitrate as u32) ]);
            second_pass.args([ "-b:a", &format!("{}K", audio_bitrate as u32) ]);
            second_pass.args([ "-pass", "2" ]);
            second_pass.arg(&output_path);

            self.run_twopass_compression(first_pass, second_pass, channels);
        } else {
            match self.audio_quality {
                Quality::Original => &mut process,
                Quality::Good     => process.args(["-b:a", "64K", "-ar", "32K"]), 
                Quality::Medium   => process.args(["-b:a", "32K", "-ar", "24K"]), 
                Quality::Bad      => process.args(["-b:a", "16K", "-ar", "16K"]), 
                Quality::Poop     => process.args(["-b:a", "8K",  "-ar", "8K"]),  
            };

            match self.video_quality {
                Quality::Original => &mut process,
                Quality::Good     => process.args(["-b:v", "1024K" ]),
                Quality::Medium   => process.args(["-b:v", "512K" ]),
                Quality::Bad      => process.args(["-b:v", "256K" ]),
                Quality::Poop     => process.args(["-b:v", "128K" ]),
            };

            process.arg(&output_path);
            self.run_compression(process, channels);
        }
    }

    pub(super) fn compress_audio(&mut self, channels: ThreadChannels) {
        let Some(ffmpeg) = get_command_string("ffmpeg") else {
            channels.send_error("FFMPEG could not be found. Please re-run the app to download required dependencies");
            return;
        };
        let mut process = Command::new(&ffmpeg);

        process.args([
            "-progress", "pipe:1",
            "-i", &self.input_path,
            "-y",
        ]);

        if self.use_output_file_size {
            let Ok(expected_size) = self.output_file_size.parse::<f32>() else {
                channels.send_error("Failed to read the output file size.");
                return;
            };

            const CONVERTION_RATE: f32 = (1024.0 * 1024.0) / (1000.0 * 1000.0);
            let size_in_mib = expected_size * CONVERTION_RATE;

            let Some(media_string) = get_media_info(&self.input_path) else { 
                channels.send_error("Failed to retreive media info about thea provided file.");
                return;
            }; 

            let Some(media_duration) = extract_media_duration(&media_string) else {
                channels.send_error("Failed to retreive media length from the provided file.");
                return;
            }; 
            let duration_in_secs = media_duration as f32 / 1000.0;

            let audio_bitrate = (size_in_mib * 8388.608) / duration_in_secs;
            process.args(["-ab", &format!("{audio_bitrate}K") ]);
        } else {
            match self.audio_quality {
                Quality::Original => &mut process,
                Quality::Good     => process.args(["-ab", "64K", "-ar", "32K"]),
                Quality::Medium   => process.args(["-ab", "32K", "-ar", "24K"]),
                Quality::Bad      => process.args(["-ab", "16K", "-ar", "16K"]),
                Quality::Poop     => process.args(["-ab", "8K",  "-ar", "8K"]),
            };
        }

        let Some(output_path) = self.get_output_path() else {
            channels.send_error("Failed to construct output path for selected input.");
            return;
        };
        process.arg(output_path);

        self.run_compression(process, channels);
    }

    fn run_twopass_compression(&mut self, mut first_pass: Command, mut second_pass: Command, channels: ThreadChannels) {
        let Some(media_string) = get_media_info(&self.input_path) else { 
            channels.send_error("Failed to retreive media info from the provided file.");
            return;
        }; 

        let Some(media_duration) = extract_media_duration(&media_string) else {
            channels.send_error("Failed to retreive media length from the provided file.");
            return;
        }; 

        // Total duration is doubled since there is a need to run ffmpeg twice.
        let total_duration = media_duration * 2;

        first_pass.stdout(std::process::Stdio::piped());
        second_pass.stdout(std::process::Stdio::piped());

        // Start the first pass
        let Ok(mut process) = first_pass.spawn() else {
            channels.send_error("Failed to start the first pass of the convertion process.");
            return;
        };

        std::thread::spawn(move || {
            let process_result = loop {
                if let Ok(()) = channels.work_abort_rx.try_recv() {
                    let _ = process.kill();
                    channels.send_info("Convertion process was canceled");
                    return;
                }

                let Ok(wait_result) = process.try_wait() else {
                    channels.send_error("Convertion unexpectedly stopped");
                    return;
                };

                if let Some(status) = wait_result {
                    break status;
                }

                let stdout = process.stdout.as_mut().unwrap();
                let mut byte_buffer = [0u8; 1024];

                use std::io::Read;
                let _bytes_read = stdout.read(&mut byte_buffer);
                let string_output = String::from_utf8_lossy(&byte_buffer);

                let out_time = "out_time_ms=";
                let Some(index) = string_output.find(out_time) else {
                    continue;
                };

                let new_idx = index + out_time.len();
                let duration_string = string_output[new_idx..].split('\n').next().unwrap();
                let Ok(current_duration) = duration_string.parse::<u64>() else {
                    continue;
                };

                let current_duration = current_duration as f32 / 1000.0;
                let new_progress = current_duration / total_duration as f32;

                let _ = channels.work_progress_tx.send(new_progress);

                println!(
                    "duration_string: {}, total_duration: {}, new_progress: {}",
                    duration_string, total_duration, new_progress
                );
            };

            if !process_result.success() {
                channels.send_error("Convertion process failed");
                return;
            };

            // Start the second pass
            let Ok(mut process) = second_pass.spawn() else {
                channels.send_error("Failed to start the second pass of the convertion process.");
                return;
            };

            let process_result = loop {
                if let Ok(()) = channels.work_abort_rx.try_recv() {
                    let _ = process.kill();
                    channels.send_info("Convertion process was canceled");
                    return;
                }

                let Ok(wait_result) = process.try_wait() else {
                    channels.send_error("Convertion unexpectedly stopped");
                    return;
                };

                if let Some(status) = wait_result {
                    break status;
                }

                let stdout = process.stdout.as_mut().unwrap();
                let mut byte_buffer = [0u8; 1024];

                use std::io::Read;
                let _bytes_read = stdout.read(&mut byte_buffer);
                let string_output = String::from_utf8_lossy(&byte_buffer);

                let out_time = "out_time_ms=";
                let Some(index) = string_output.find(out_time) else {
                    continue;
                };

                let new_idx = index + out_time.len();
                let duration_string = string_output[new_idx..].split('\n').next().unwrap();
                let Ok(current_duration) = duration_string.parse::<u64>() else {
                    continue;
                };

                // Adding media duration here since this is the second pass.
                let mut current_duration = current_duration as f32 / 1000.0;
                current_duration += media_duration as f32;

                let new_progress = current_duration / total_duration as f32;
                let _ = channels.work_progress_tx.send(new_progress);

                println!(
                    "duration_string: {}, total_duration: {}, new_progress: {}",
                    duration_string, total_duration, new_progress
                );
            };

            if process_result.success() {
                channels.send_success("Convertion finished successfully");
            } else {
                channels.send_error("Convertion process failed");
            };
        });
    }

    fn run_compression(&mut self, mut process: Command, channels: ThreadChannels) {
        let Some(media_string) = get_media_info(&self.input_path) else { 
            channels.send_error("Failed to retreive media info from the provided file.");
            return;
        }; 

        let Some(total_duration) = extract_media_duration(&media_string) else {
            channels.send_error("Failed to retreive media length from the provided file.");
            return;
        }; 

        // Finally start the convertion.
        process.stdout(std::process::Stdio::piped());
        let Ok(mut process) = process.spawn() else {
            channels.send_error("Failed to start the the convertion process.");
            return;
        };

        std::thread::spawn(move || {
            let process_result = loop {
                if let Ok(()) = channels.work_abort_rx.try_recv() {
                    let _ = process.kill();
                    channels.send_info("Convertion process was canceled");
                    return;
                }

                let Ok(wait_result) = process.try_wait() else {
                    channels.send_error("Convertion unexpectedly stopped");
                    return;
                };

                if let Some(status) = wait_result {
                    break status;
                }

                let stdout = process.stdout.as_mut().unwrap();

                use std::io::Read;
                let mut byte_buffer = [0u8; 1024];
                let _bytes_read = stdout.read(&mut byte_buffer);

                let string_output = String::from_utf8_lossy(&byte_buffer);

                let out_time = "out_time_ms=";
                let Some(index) = string_output.find(out_time) else {
                    continue;
                };

                let new_idx = index + out_time.len();
                let duration_string = string_output[new_idx..].split('\n').next().unwrap();
                let Ok(current_duration) = duration_string.parse::<u64>() else {
                    continue;
                };

                let new_progress = current_duration as f32 / total_duration as f32 / 1000.0;
                let _ = channels.work_progress_tx.send(new_progress);

                println!(
                    "duration_string: {}, total_duration: {}, new_progress: {}",
                    duration_string, total_duration, new_progress
                );
               
            };

            if process_result.success() {
                channels.send_success("Convertion finished successfully");
            } else {
                channels.send_error("Convertion process failed");
            };
        });
    }
}
