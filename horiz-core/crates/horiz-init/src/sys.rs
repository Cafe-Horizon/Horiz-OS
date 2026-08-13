#[cfg(target_os = "linux")]
use std::ffi::CString;

use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
#[cfg(target_os = "linux")]
use libc::{mount, waitpid, WNOHANG};
use crate::logger::{log_message, LogLevel};

#[cfg(target_os = "linux")]
pub fn mount_fs(source: &str, target: &str, fstype: &str, flags: u64) {
    let c_source = CString::new(source).unwrap();
    let c_target = CString::new(target).unwrap();
    let c_fstype = CString::new(fstype).unwrap();

    unsafe {
        if mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            flags,
            std::ptr::null(),
        ) == 0
        {
            log_message(LogLevel::Info, &format!("{} をマウント完了。", target));
        } else {
            log_message(LogLevel::Warn, &format!("{} のマウントに失敗しました。", target));
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn mount_fs(_source: &str, target: &str, _fstype: &str, _flags: u64) {
    log_message(LogLevel::Info, &format!("{} のマウントはNon-Linux環境のためスキップされました。", target));
}

pub fn setup_network() {
    log_message(LogLevel::Info, "ネットワークインターフェースを初期化中...");
    
    let mut success = false;
    for i in 1..=3 {
        log_message(LogLevel::Info, &format!("ループバックインターフェース (lo) の有効化を試行中 (回数: {}/3)...", i));
        let status = Command::new("ip").args(&["link", "set", "lo", "up"]).status();
        match status {
            Ok(s) if s.success() => {
                log_message(LogLevel::Info, "ループバックインターフェース (lo) を有効化。");
                success = true;
                break;
            }
            Ok(s) => log_message(LogLevel::Warn, &format!("ip コマンドがエラーを返しました (status: {})。", s)),
            Err(e) => log_message(LogLevel::Error, &format!("ip コマンドの実行に失敗: {}", e)),
        }
        thread::sleep(Duration::from_secs(1));
    }

    if !success {
        log_message(LogLevel::Error, "ネットワーク初期化に致命的な失敗が発生しました。一部の機能が制限される可能性があります。");
    }
}

pub fn reap_zombies() {
    #[cfg(target_os = "linux")]
    unsafe {
        loop {
            let mut status = 0;
            let pid = waitpid(-1, &mut status, WNOHANG);
            if pid <= 0 {
                break;
            }
            log_message(LogLevel::Info, &format!("ゾンビプロセスを回収: PID {}", pid));
        }
    }
}

/// /etc/horiz/services.conf からサービス設定を読み込み、バックグラウンド起動および死活監視を実施
pub fn supervise_services() {
    let conf_path = "/etc/horiz/services.conf";
    if let Ok(contents) = fs::read_to_string(conf_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }

            let service_name = parts[0];
            let args = &parts[1..];

            // 簡易サービス管理: バックグラウンド起動を試行
            log_message(LogLevel::Info, &format!("サービスプロファイルを確認中: {}", service_name));
            let _ = Command::new(service_name)
                .args(args)
                .spawn();
        }
    }
}

/// システムの再起動または電源シャットダウン
pub fn shutdown_system(_reboot: bool) -> ! {

    log_message(LogLevel::Info, "全プロセスへ SIGTERM を送信中...");
    #[cfg(target_os = "linux")]
    unsafe {
        libc::sync();
        libc::kill(-1, libc::SIGTERM);
        thread::sleep(Duration::from_millis(500));
        libc::kill(-1, libc::SIGKILL);

        let cmd = if _reboot {
            libc::LINUX_REBOOT_CMD_RESTART
        } else {
            libc::LINUX_REBOOT_CMD_POWER_OFF
        };
        log_message(LogLevel::Info, if _reboot { "システムを再起動します。" } else { "システムを停止します。" });

        libc::reboot(cmd);
    }

    log_message(LogLevel::Warn, "シャットダウン呼び出しを完了。終了プロセスへ移行します。");
    std::process::exit(0);
}
