use std::fs;
#[cfg(target_os = "linux")]
use libc::{MS_NODEV, MS_NOEXEC, MS_NOSUID, SIGCHLD, SIG_DFL, signal};

mod logger;
mod session;
mod sys;

use logger::{log_message, LogLevel};
use session::{login_prompt, run_session};
use sys::{mount_fs, setup_network};

fn main() {
    println!("--- HorizOS Core Initializing (Enhanced Security) ---");

    // シグナルハンドリングの初期化
    #[cfg(target_os = "linux")]
    unsafe {
        signal(SIGCHLD, SIG_DFL);
    }

    // 1. 仮想ファイルシステムのマウント (セキュリティ強化)
    #[cfg(target_os = "linux")]
    {
        mount_fs("proc", "/proc", "proc", MS_NOSUID | MS_NODEV | MS_NOEXEC);
        mount_fs("sysfs", "/sys", "sysfs", MS_NOSUID | MS_NODEV | MS_NOEXEC);
        mount_fs("devtmpfs", "/dev", "devtmpfs", MS_NOSUID | MS_NOEXEC);
        mount_fs("tmpfs", "/tmp", "tmpfs", MS_NOSUID | MS_NODEV | MS_NOEXEC);
    }

    #[cfg(not(target_os = "linux"))]
    {
        mount_fs("proc", "/proc", "proc", 0);
        mount_fs("sysfs", "/sys", "sysfs", 0);
        mount_fs("devtmpfs", "/dev", "devtmpfs", 0);
        mount_fs("tmpfs", "/tmp", "tmpfs", 0);
    }

    // 必須ディレクトリの作成
    let _ = fs::create_dir_all("/var/log");

    // 2. ネットワークセットアップ
    setup_network();

    log_message(LogLevel::Info, "システム初期化完了。セキュリティプロファイル適用済。");

    loop {
        let (user, uid, gid) = login_prompt();
        run_session(&user, uid, gid);
    }
}


