use std::env;
#[cfg(unix)]
use std::ffi::CString;

use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use horiz_auth;

use crate::logger::{log_message, LogLevel};
use crate::sys::reap_zombies;

pub fn get_user_info(username: &str) -> (u32, u32) {
    if let Ok(contents) = fs::read_to_string("/etc/passwd") {
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 && parts[0] == username {
                if let (Ok(uid), Ok(gid)) = (parts[2].parse::<u32>(), parts[3].parse::<u32>()) {
                    return (uid, gid);
                }
            }
        }
    }
    if username == "root" { (0, 0) } else { (1000, 1000) }
}

pub fn read_password() -> String {
    #[cfg(unix)]
    {
        let mut term: libc::termios = unsafe { std::mem::zeroed() };
        unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut term); }
        let mut term_hidden = term;
        term_hidden.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &term_hidden); }
        
        let mut pass = String::new();
        io::stdin().read_line(&mut pass).unwrap();
        
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &term); }
        println!();
        pass.trim().to_string()
    }
    #[cfg(not(unix))]
    {
        let mut pass = String::new();
        io::stdin().read_line(&mut pass).unwrap();
        println!();
        pass.trim().to_string()
    }
}

pub fn login_prompt() -> (String, u32, u32) {
    loop {
        println!("\n--- HorizOS Login ---");
        print!("username: ");
        io::stdout().flush().unwrap();
        let mut username = String::new();
        io::stdin().read_line(&mut username).unwrap();
        let username = username.trim().to_string();

        if username.is_empty() { continue; }

        if username == "reboot" {
            crate::sys::shutdown_system(true);
        } else if username == "poweroff" || username == "shutdown" {
            crate::sys::shutdown_system(false);
        }


        print!("password: ");
        io::stdout().flush().unwrap();
        
        let password = read_password();

        match horiz_auth::verify_login(&username, &password) {
            Ok(true) => {
                log_message(LogLevel::Info, &format!("認証成功。ユーザー: {}", username));
                log_message(LogLevel::Audit, &format!("Successful login for user: {}", username));
                
                let (uid, gid) = get_user_info(&username);
                return (username, uid, gid);
            }
            Ok(false) => {
                log_message(LogLevel::Warn, &format!("ログイン失敗。ユーザー: {}", username));
                log_message(LogLevel::Audit, &format!("Failed login attempt for user: {}", username));
            }
            Err(e) => {
                log_message(LogLevel::Error, &format!("認証システムエラー: {}", e));
            }
        }
    }
}

pub fn run_session(user: &str, uid: u32, gid: u32) {
    unsafe { env::set_var("USER", user); }
    log_message(LogLevel::Info, &format!("ユーザーステータスを開始: {} (UID: {}, GID: {})", user, uid, gid));

    loop {
        reap_zombies();
        
        #[cfg(unix)]
        unsafe {
            let pid = libc::fork();
            if pid == 0 {
                if uid != 0 {
                    libc::setgid(gid);
                    libc::setuid(uid);
                }
                
                let cmd = CString::new("/bin/sh").unwrap();
                let arg0 = CString::new("sh").unwrap();
                let args = [arg0.as_ptr(), std::ptr::null()];
                
                libc::execv(cmd.as_ptr(), args.as_ptr());
                libc::_exit(1);
            } else if pid > 0 {
                let mut status = 0;
                libc::waitpid(pid, &mut status, 0);
                log_message(LogLevel::Warn, &format!("セッションが終了しました (status: {})。再起動します。", status));
                log_message(LogLevel::Audit, &format!("Session ended for user: {} (status: {})", user, status));
            } else {
                log_message(LogLevel::Error, "フォークに失敗しました。");
                thread::sleep(Duration::from_secs(1));
            }
        }

        #[cfg(not(unix))]
        {
            log_message(LogLevel::Warn, "Non-Unix環境のためセッションプロセス実行をループ停止します。");
            thread::sleep(Duration::from_secs(5));
        }
    }
}
