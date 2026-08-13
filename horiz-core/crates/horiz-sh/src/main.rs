use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;

fn expand_vars(input: &str) -> String {
    let mut result = String::new();
    let mut parts = input.split('$');

    if let Some(first) = parts.next() {
        result.push_str(first);
    }

    for part in parts {
        let var_name_len = part.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
        if var_name_len > 0 {
            let var_name = &part[..var_name_len];
            let rest = &part[var_name_len..];
            if let Ok(val) = env::var(var_name) {
                result.push_str(&val);
            }
            result.push_str(rest);
        } else {
            result.push('$');
            result.push_str(part);
        }
    }
    result
}

fn append_history(line: &str) {
    if let Ok(home) = env::var("HOME").or_else(|_| env::var("USER").map(|u| format!("/home/{}", u))) {
        let hist_path = Path::new(&home).join(".horiz_history");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(hist_path) {
            let _ = writeln!(f, "{}", line);
        }
    } else if let Ok(mut f) = OpenOptions::new().create(true).append(true).open("/tmp/.horiz_history") {
        let _ = writeln!(f, "{}", line);
    }
}

fn show_history() {
    let hist_path = env::var("HOME")
        .map(|h| format!("{}/.horiz_history", h))
        .unwrap_or_else(|_| "/tmp/.horiz_history".to_string());

    if let Ok(f) = fs::File::open(&hist_path) {
        let reader = BufReader::new(f);
        for (idx, l) in reader.lines().flatten().enumerate() {
            println!("{:>5}  {}", idx + 1, l);
        }
    }
}

#[cfg(unix)]
fn execute_pipeline(pipeline: Vec<&str>) {
    use std::ffi::CString;

    let num_cmds = pipeline.len();
    let mut pipes = Vec::new();

    for _ in 0..num_cmds.saturating_sub(1) {
        let mut fds = [0i32; 2];
        unsafe {
            if libc::pipe(fds.as_mut_ptr()) != 0 {
                eprintln!("パイプの作成に失敗しました");
                return;
            }
        }
        pipes.push(fds);
    }

    let mut children = Vec::new();

    for (i, cmd_str) in pipeline.into_iter().enumerate() {
        let mut tokens: Vec<&str> = cmd_str.split_whitespace().collect();
        if tokens.is_empty() { continue; }

        let mut infile: Option<&str> = None;
        let mut outfile: Option<&str> = None;
        let mut append = false;
        let mut clean_tokens = Vec::new();

        let mut idx = 0;
        while idx < tokens.len() {
            match tokens[idx] {
                "<" => {
                    if idx + 1 < tokens.len() {
                        infile = Some(tokens[idx + 1]);
                        idx += 2;
                        continue;
                    }
                }
                ">" => {
                    if idx + 1 < tokens.len() {
                        outfile = Some(tokens[idx + 1]);
                        append = false;
                        idx += 2;
                        continue;
                    }
                }
                ">>" => {
                    if idx + 1 < tokens.len() {
                        outfile = Some(tokens[idx + 1]);
                        append = true;
                        idx += 2;
                        continue;
                    }
                }
                _ => {
                    clean_tokens.push(tokens[idx]);
                }
            }
            idx += 1;
        }

        if clean_tokens.is_empty() { continue; }

        unsafe {
            let pid = libc::fork();
            if pid == 0 {
                // 子プロセス: シグナルをデフォルトに復元
                libc::signal(libc::SIGINT, libc::SIG_DFL);

                // パイプ接続 (標準入力)
                if i > 0 {
                    libc::dup2(pipes[i - 1][0], libc::STDIN_FILENO);
                }
                // パイプ接続 (標準出力)
                if i < num_cmds - 1 {
                    libc::dup2(pipes[i][1], libc::STDOUT_FILENO);
                }

                // 全パイプ FDs を閉じる
                for p in &pipes {
                    libc::close(p[0]);
                    libc::close(p[1]);
                }

                // リダイレクト処理
                if let Some(in_path) = infile {
                    let c_in = CString::new(in_path).unwrap();
                    let fd = libc::open(c_in.as_ptr(), libc::O_RDONLY);
                    if fd < 0 {
                        eprintln!("{}: ファイルが開けません", in_path);
                        libc::_exit(1);
                    }
                    libc::dup2(fd, libc::STDIN_FILENO);
                    libc::close(fd);
                }

                if let Some(out_path) = outfile {
                    let c_out = CString::new(out_path).unwrap();
                    let flags = if append {
                        libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND
                    } else {
                        libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC
                    };
                    let fd = libc::open(c_out.as_ptr(), flags, 0o644);
                    if fd < 0 {
                        eprintln!("{}: ファイルの書き込みオープンに失敗しました", out_path);
                        libc::_exit(1);
                    }
                    libc::dup2(fd, libc::STDOUT_FILENO);
                    libc::close(fd);
                }

                // コマンド実行
                let c_cmd = CString::new(clean_tokens[0]).unwrap();
                let c_args: Vec<CString> = clean_tokens.iter().map(|s| CString::new(*s).unwrap()).collect();
                let mut arg_ptrs: Vec<*const libc::c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
                arg_ptrs.push(std::ptr::null());

                libc::execvp(c_cmd.as_ptr(), arg_ptrs.as_ptr());

                // /bin/ フォールバック
                let fallback = format!("/bin/{}", clean_tokens[0]);
                if let Ok(c_fb) = CString::new(fallback) {
                    libc::execv(c_fb.as_ptr(), arg_ptrs.as_ptr());
                }

                eprintln!("{}: コマンドの実行に失敗しました", clean_tokens[0]);
                libc::_exit(127);
            } else if pid > 0 {
                children.push(pid);
            }
        }
    }

    // 親プロセス: 不要なパイプ FDs を閉じて待機
    for p in &pipes {
        unsafe {
            libc::close(p[0]);
            libc::close(p[1]);
        }
    }

    for pid in children {
        let mut status = 0;
        unsafe {
            libc::waitpid(pid, &mut status, 0);
        }
    }
}

#[cfg(not(unix))]
fn execute_pipeline(pipeline: Vec<&str>) {
    for cmd_str in pipeline {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if !parts.is_empty() {
            let _ = Command::new(parts[0]).args(&parts[1..]).status();
        }
    }
}

fn main() {
    #[cfg(unix)]
    unsafe {
        // シェル自身は SIGINT (Ctrl+C) を無視する
        libc::signal(libc::SIGINT, libc::SIG_IGN);
    }

    let hostname = fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "horiz".to_string());

    println!("--- HorizOS Shell v1.5.0 (Pipeline & Signal Enhanced) ---");

    loop {
        let user = env::var("USER").unwrap_or_else(|_| "root".to_string());
        let cwd = env::current_dir().unwrap_or_else(|_| Path::new("/").to_path_buf());
        let cwd_display = cwd.to_string_lossy();

        print!("[{}@{}] {} # ", user, hostname, cwd_display);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).unwrap() == 0 {
            break; // EOF
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        append_history(input);

        // 変数展開
        let expanded_input = expand_vars(input);

        let pipeline: Vec<&str> = expanded_input.split('|').collect();

        // パイプなし＆単一の組み込みコマンドの処理
        if pipeline.len() == 1 {
            let parts: Vec<&str> = pipeline[0].split_whitespace().collect();
            if parts.is_empty() { continue; }
            let cmd = parts[0];
            let args = &parts[1..];

            match cmd {
                "exit" => break,
                "cd" => {
                    let new_dir = args.get(0).copied().unwrap_or("/");
                    if let Err(e) = env::set_current_dir(Path::new(new_dir)) {
                        eprintln!("cd: {}", e);
                    }
                    continue;
                }
                "export" => {
                    for arg in args {
                        let kv: Vec<&str> = arg.splitn(2, '=').collect();
                        if kv.len() == 2 {
                            unsafe { env::set_var(kv[0], kv[1]); }
                        }
                    }
                    continue;
                }
                "unset" => {
                    for arg in args {
                        unsafe { env::remove_var(arg); }
                    }
                    continue;
                }
                "history" => {
                    show_history();
                    continue;
                }
                "whoami" => {
                    println!("{}", user);
                    continue;
                }
                "version" => {
                    println!("HorizOS Shell v1.5.0 (Pipeline & Multi-Builtin Edition)");
                    continue;
                }
                _ => {}
            }
        }

        execute_pipeline(pipeline);
    }
}
