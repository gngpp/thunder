use anyhow::Result;
use daemonize::Daemonize;
use std::{
    fs::{File, Permissions},
    os::unix::prelude::PermissionsExt,
};
use std::{
    io::{self, BufRead},
    path::Path,
};

const PID_PATH: &str = "/var/run/thunder.pid";
const DEFAULT_STDOUT_PATH: &str = "/var/run/thunder.out";
const DEFAULT_STDERR_PATH: &str = "/var/run/thunder.err";
const DEFAULT_WORK_DIR: &str = "/";

/// Check if the user is root
pub fn check_root() {
    if !nix::unistd::Uid::effective().is_root() {
        println!("You must run this executable with root permissions");
        std::process::exit(-1)
    }
}

/// Get the pid of the daemon
pub fn get_pid() -> Option<i32> {
    if let Ok(data) = std::fs::read(PID_PATH) {
        let binding = String::from_utf8(data).expect("pid file is not utf8");
        return Some(binding.trim().parse().expect("pid file is not a number"));
    }
    None
}

/// Start the daemon
pub fn start() -> Result<()> {
    if let Some(pid) = get_pid() {
        println!("Thunder is already running with pid: {pid}");
        return Ok(());
    }

    check_root();

    let pid_file = File::create(PID_PATH)?;
    pid_file.set_permissions(Permissions::from_mode(0o755))?;

    let stdout = File::create(DEFAULT_STDOUT_PATH)?;
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let stderr = File::create(DEFAULT_STDERR_PATH)?;
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let daemonize = Daemonize::new()
        .pid_file(PID_PATH) // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory(DEFAULT_WORK_DIR) // for default behaviour.
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    if let Some(err) = daemonize.start().err() {
        eprintln!("Error: {err}")
    }

    Ok(())
}

/// Stop the daemon
pub fn stop() -> Result<()> {
    use nix::sys::signal;
    use nix::unistd::Pid;

    check_root();

    if let Some(pid) = get_pid() {
        for _ in 0..360 {
            if signal::kill(Pid::from_raw(pid), signal::SIGINT).is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1))
        }
        let _ = std::fs::remove_file(PID_PATH);
    }

    Ok(())
}

/// Show the status of the daemon
pub fn status() -> Result<()> {
    use sysinfo::System;
    match get_pid() {
        Some(pid) => {
            let mut sys = System::new();

            // First we update all information of our `System` struct.
            sys.refresh_all();

            // Display processes ID
            let process = sys
                .processes()
                .into_iter()
                .find(|(raw_pid, _)| raw_pid.as_u32().eq(&(pid as u32)))
                .ok_or_else(|| anyhow::anyhow!("thunder is not running"))?;

            println!("{:<6} {:<6}  {:<6}", "PID", "CPU(%)", "MEM(MB)");
            println!(
                "{:<6}   {:<6.1}  {:<6.1}",
                process.0,
                process.1.cpu_usage(),
                (process.1.memory() as f64) / 1024.0 / 1024.0
            );
        }
        None => println!("thunder is not running"),
    }
    Ok(())
}

/// Show the log of the daemon
pub fn log() -> Result<()> {
    fn read_and_print_file(file_path: &Path, placeholder: &str) -> Result<()> {
        if !file_path.exists() {
            return Ok(());
        }

        // Check if the file is empty before opening it
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() == 0 {
            return Ok(());
        }

        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);
        let mut start = true;

        for line in reader.lines() {
            if let Ok(content) = line {
                if start {
                    start = false;
                    println!("{placeholder}");
                }
                println!("{}", content);
            } else if let Err(err) = line {
                eprintln!("Error reading line: {}", err);
            }
        }

        Ok(())
    }

    let stdout_path = Path::new(DEFAULT_STDOUT_PATH);
    read_and_print_file(stdout_path, "STDOUT>")?;

    let stderr_path = Path::new(DEFAULT_STDERR_PATH);
    read_and_print_file(stderr_path, "STDERR>")?;

    Ok(())
}
