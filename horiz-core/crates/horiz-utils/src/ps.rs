use std::fs;
use std::io;
use std::path::Path;

pub fn ps(_args: Vec<String>) -> io::Result<()> {
    println!("{:>7} {:<10} {:<15} {}", "PID", "TTY", "STAT", "COMMAND");

    let proc_dir = Path::new("/proc");
    if !proc_dir.exists() {
        println!("{:>7} {:<10} {:<15} {}", std::process::id(), "?", "R", "horiz-utils");
        return Ok(());
    }

    if let Ok(entries) = fs::read_dir(proc_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.chars().all(|c| c.is_ascii_digit()) {
                let pid = name_str;
                let comm_path = entry.path().join("comm");
                let cmdline_path = entry.path().join("cmdline");
                let stat_path = entry.path().join("stat");

                let comm = fs::read_to_string(&comm_path)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "?".to_string());

                let cmdline = fs::read_to_string(&cmdline_path)
                    .map(|s| s.replace('\0', " ").trim().to_string())
                    .unwrap_or_default();

                let display_cmd = if cmdline.is_empty() { format!("[{}]", comm) } else { cmdline };

                let state = fs::read_to_string(&stat_path)
                    .map(|s| {
                        s.split_whitespace()
                            .nth(2)
                            .unwrap_or("R")
                            .to_string()
                    })
                    .unwrap_or_else(|_| "S".to_string());

                println!("{:>7} {:<10} {:<15} {}", pid, "?", state, display_cmd);
            }
        }
    }

    Ok(())
}
