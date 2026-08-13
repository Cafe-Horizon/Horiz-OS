use std::env;
use std::io::{self, Write};

fn read_password(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

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

fn main() {
    let args: Vec<String> = env::args().collect();
    let current_user = env::var("USER").unwrap_or_else(|_| "root".to_string());
    let target_user = args.get(1).unwrap_or(&current_user);

    println!("{} のパスワードを変更中。", target_user);

    let old_pwd = read_password("現在のパスワード: ");
    let new_pwd1 = read_password("新しいパスワード: ");
    let new_pwd2 = read_password("新しいパスワード (再入力): ");

    if new_pwd1 != new_pwd2 {
        eprintln!("エラー: 新しいパスワードが一致しません。");
        std::process::exit(1);
    }

    if new_pwd1.is_empty() {
        eprintln!("エラー: 空のパスワードは許可されていません。");
        std::process::exit(1);
    }

    match horiz_auth::change_password(target_user, &old_pwd, &new_pwd1) {
        Ok(true) => {
            println!("passwd: パスワードは正常に更新されました。");
        }
        Ok(false) => {
            eprintln!("passwd: 認証に失敗しました。パスワードが正しくありません。");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("passwd: システムエラー: {}", e);
            std::process::exit(1);
        }
    }
}
