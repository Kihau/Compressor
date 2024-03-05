#[cfg(test)]
mod tests {
    #[test]
    fn process_args_test() {
        let mut process = std::process::Command::new("ls");
        process.args(["-l", "-a"]);
        process.args(["-F"]);
        process.arg("-h");
        let output = process.output().unwrap().stdout;
        println!("thing: {}", String::from_utf8_lossy(&output));
    }

    //#[test]
    #[allow(dead_code)]
    fn deadlocking_test() {
        println!("Deadlock test started...");
        use std::sync::Mutex;

        // OMEGA deadge-lock
        let something = Mutex::new(0.0);
        let mut aquired_lock = something.lock().unwrap();
        *something.lock().unwrap() = 4.0;

        *aquired_lock += 1.0;
        let value = *aquired_lock;
        println!("Nothing was blocked and the value is: {value}");
    }

    #[test]
    fn threading_test() {
        println!("Threading test started...");
        let something = std::sync::Arc::new(std::sync::Mutex::new(0.0));

        let other = something.clone();
        std::thread::spawn(move || {
            *other.lock().unwrap() += 12.0;
            let mut floating = other.lock().unwrap();
            loop {
                *floating += 1.0;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            //println!("So it ended");
        });

        std::thread::sleep(std::time::Duration::from_secs(2));
    
        let trylock = something.try_lock();
        if let Ok(mutex_lock) = trylock {
            let value = *mutex_lock;
            println!("The lock was released, it's all good now and the value is {value}");
        } else {
            println!("Failed to aquire the lock, SHIT!");
        }
    }

    #[test]
    fn extraction_test() {
        let Some(info) = compressor::get_media_info("suckmynuts.mp3") else {
            panic!("rip");

        };
        println!("info is: {info}");
        let duration = compressor::extract_media_duration(&info);
        println!("duration in: {duration:?}ms");
    }
}
