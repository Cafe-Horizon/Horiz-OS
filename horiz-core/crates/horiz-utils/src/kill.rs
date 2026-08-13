use std::io;

pub fn kill(args: Vec<String>) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: kill [-signal] <pid>..."));
    }

    let mut sig = 15; // SIGTERM
    let mut pids = Vec::new();

    for arg in args {
        if arg.starts_with('-') {
            let sig_str = &arg[1..];
            if let Ok(s) = sig_str.parse::<i32>() {
                sig = s;
            } else {
                match sig_str.to_uppercase().as_str() {
                    "KILL" | "9" => sig = 9,
                    "TERM" | "15" => sig = 15,
                    "INT" | "2" => sig = 2,
                    "HUP" | "1" => sig = 1,
                    _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("不明なシグナル: {}", arg))),
                }
            }
        } else if let Ok(pid) = arg.parse::<i32>() {
            pids.push(pid);
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("無効な PID: {}", arg)));
        }
    }

    if pids.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "PID が指定されていません。"));
    }

    for pid in pids {
        #[cfg(unix)]
        unsafe {
            if libc::kill(pid, sig) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(not(unix))]
        {
            println!("Non-Unix 環境のため PID {} へのシグナル {} 送信をシミュレートしました。", pid, sig);
        }
    }

    Ok(())
}
