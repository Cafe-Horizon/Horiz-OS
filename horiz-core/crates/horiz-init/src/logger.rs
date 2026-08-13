use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// ログレベルの定義
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Audit,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Audit => "AUDIT",
        }
    }
}

/// タイムスタンプを取得 (Zero-Dependency)
pub fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 構造化ログを出力
pub fn log_message(level: LogLevel, message: &str) {
    let ts = get_timestamp();
    let log_entry = format!("[{}] [{}] {}\n", ts, level.as_str(), message);
    
    // 標準出力への報告
    match level {
        LogLevel::Error => eprintln!("{}", log_entry.trim()),
        _ => println!("{}", log_entry.trim()),
    }

    // ログファイルへの永続化 (シンボリックリンク攻撃対策)
    let log_paths = vec!["/var/log/system.log"];
    let mut target_paths = log_paths;
    if let LogLevel::Audit = level {
        target_paths.push("/var/log/audit.log");
    }

    for path in target_paths {
        // シンボリックリンクをチェックして、リンク先への意図せぬ書き込みを防止
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if metadata.file_type().is_symlink() {
                eprintln!("[警告] ログファイル {} がシンボリックリンクです。攻撃の可能性があるためスキップします。", path);
                continue;
            }
        }

        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(log_entry.as_bytes());
        }
    }
}
