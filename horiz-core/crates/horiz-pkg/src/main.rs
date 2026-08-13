use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

mod sha256;
mod sha512;
mod ed25519;
mod x25519;
mod chacha20poly1305;
mod hkdf;
mod tls;
mod base64;
mod pem;
mod x509;

const DB_PATH: &str = "/var/db/horiz-pkg/manifest.db";

fn ensure_db_dir() {
    let _ = fs::create_dir_all("/var/db/horiz-pkg");
}

fn add_to_manifest(name: &str, url: &str, target_path: &str, hash: &str) -> io::Result<()> {
    ensure_db_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let entry = format!("{}|{}|{}|{}|{}\n", name, url, target_path, hash, timestamp);
    
    // 重複を削除してから追記
    let _ = remove_from_manifest(name);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(DB_PATH)?;
    file.write_all(entry.as_bytes())?;
    Ok(())
}

fn remove_from_manifest(name: &str) -> io::Result<Option<String>> {
    ensure_db_dir();
    if !Path::new(DB_PATH).exists() { return Ok(None); }

    let file = fs::File::open(DB_PATH)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut removed_path = None;

    for line in reader.lines().flatten() {
        let parts: Vec<&str> = line.split('|').collect();
        if !parts.is_empty() && parts[0] == name {
            if parts.len() >= 3 {
                removed_path = Some(parts[2].to_string());
            }
            continue; // スキップ (削除)
        }
        lines.push(line);
    }

    let mut out = fs::File::create(DB_PATH)?;
    for l in lines {
        writeln!(out, "{}", l)?;
    }

    Ok(removed_path)
}

fn list_manifest() -> io::Result<()> {
    ensure_db_dir();
    if !Path::new(DB_PATH).exists() {
        println!("インストールされているパッケージはありません。");
        return Ok(());
    }

    let file = fs::File::open(DB_PATH)?;
    let reader = BufReader::new(file);

    println!("{:<15} {:<10} {:<25} {}", "NAME", "STATUS", "PATH", "URL");
    println!("{}", "-".repeat(70));

    let mut count = 0;
    for line in reader.lines().flatten() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let name = parts[0];
            let url = parts[1];
            let path = parts[2];
            let status = if Path::new(path).exists() { "OK" } else { "MISSING" };
            println!("{:<15} {:<10} {:<25} {}", name, status, path, url);
            count += 1;
        }
    }

    if count == 0 {
        println!("インストールされているパッケージはありません。");
    }
    Ok(())
}

struct Args {
    subcommand: String,
    url: String,
    name: String,
    pubkey: String,
    trust_store: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let env_args: Vec<String> = env::args().collect();
    if env_args.len() < 2 {
        return Err("使用法: horiz-pkg <install|list|remove|status> [options]".into());
    }

    let mut subcommand = env_args[1].clone();
    let mut start_idx = 2;

    // 後体互換性: フラグ (-u など) で直接始まっている場合は install モード
    if subcommand.starts_with('-') {
        subcommand = "install".to_string();
        start_idx = 1;
    }

    let mut url = String::new();
    let mut name = String::new();
    let mut pubkey = "/bin/pkg.pub".to_string();
    let mut trust_store = Vec::new();

    let mut i = start_idx;
    while i < env_args.len() {
        match env_args[i].as_str() {
            "-u" | "--url" => {
                if i + 1 < env_args.len() {
                    url = env_args[i + 1].clone();
                    i += 2;
                } else { return Err("Missing value for --url".into()); }
            }
            "-n" | "--name" => {
                if i + 1 < env_args.len() {
                    name = env_args[i + 1].clone();
                    i += 2;
                } else { return Err("Missing value for --name".into()); }
            }
            "-p" | "--pubkey" => {
                if i + 1 < env_args.len() {
                    pubkey = env_args[i + 1].clone();
                    i += 2;
                } else { return Err("Missing value for --pubkey".into()); }
            }
            "--trust" => {
                if i + 1 < env_args.len() {
                    trust_store.push(env_args[i + 1].clone());
                    i += 2;
                } else { return Err("Missing value for --trust".into()); }
            }
            _ => {
                if subcommand == "remove" && name.is_empty() {
                    name = env_args[i].clone();
                    i += 1;
                } else {
                    return Err(format!("Unknown argument: {}", env_args[i]));
                }
            }
        }
    }

    if trust_store.is_empty() && Path::new("/etc/horiz/certs.pem").exists() {
        trust_store.push("/etc/horiz/certs.pem".to_string());
    }

    Ok(Args { subcommand, url, name, pubkey, trust_store })
}

const MAX_RESPONSE_SIZE: usize = 100 * 1024 * 1024; // 100MB

fn http_get(url: &str, trust_store_keys: &[[u8; 32]]) -> io::Result<Vec<u8>> {
    if url.starts_with("https://") {
        return tls::https_get(url, trust_store_keys);
    }
    let stripped = url.strip_prefix("http://").ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Only http:// and https:// are supported"))?;
    let (host_port, path) = match stripped.find('/') {
        Some(i) => (&stripped[..i], &stripped[i..]),
        None => (stripped, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(i) => (&host_port[..i], host_port[i+1..].parse::<u16>().map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid port"))?),
        None => (host_port, 80),
    };

    let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = stream.read(&mut buffer)?;
        if n == 0 { break; }
        if response.len() + n > MAX_RESPONSE_SIZE {
            return Err(io::Error::new(io::ErrorKind::Other, "[拒否] レスポンスが制限サイズ (100MB) を超えました。DoS攻撃を検知したため中断します。"));
        }
        response.extend_from_slice(&buffer[..n]);
    }

    if let Some(pos) = response.windows(4).position(|w| w == b"\r\n\r\n") {
        let header = String::from_utf8_lossy(&response[..pos]);
        if !header.contains("200 OK") {
            return Err(io::Error::new(io::ErrorKind::Other, format!("HTTP Error: {}", header.lines().next().unwrap_or("Unknown"))));
        }
        Ok(response[pos + 4..].to_vec())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid HTTP response"))
    }
}

fn main() -> io::Result<()> {
    let args = parse_args().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    match args.subcommand.as_str() {
        "list" | "status" => {
            list_manifest()?;
            return Ok(());
        }
        "remove" => {
            if args.name.is_empty() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "[エラー] 削除するパッケージ名を指定してください。"));
            }
            if let Some(path) = remove_from_manifest(&args.name)? {
                if Path::new(&path).exists() {
                    fs::remove_file(&path)?;
                    println!("[報告] パッケージバイナリを削除しました: {}", path);
                }
                println!("[報告] パッケージ {} の削除を完了しました。", args.name);
            } else {
                let target_path = format!("/bin/{}", args.name);
                if Path::new(&target_path).exists() {
                    fs::remove_file(&target_path)?;
                    println!("[報告] /bin/ 内のバイナリを削除しました: {}", target_path);
                } else {
                    println!("[警告] パッケージ {} がマニフェストで見つかりませんでした。", args.name);
                }
            }
            return Ok(());
        }
        "install" => {
            if args.url.is_empty() || args.name.is_empty() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: horiz-pkg install --url <URL> --name <NAME> [--pubkey <PATH>] [--trust <CA_PEM_PATH>]"));
            }

            if args.name.contains('/') || args.name.contains('\\') || args.name.contains("..") {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "[エラー] 無効なパッケージ名です (パストラバーサルの試行を検知)。"));
            }

            let target_path = format!("/bin/{}", args.name);
            let tmp_path = format!("/tmp/{}.tmp", args.name);
            let sig_url = format!("{}.sig", args.url);

            let mut trust_store_keys = Vec::new();
            for path in &args.trust_store {
                if let Ok(content) = fs::read_to_string(path) {
                    let pems = pem::parse(&content);
                    for p in pems {
                        if p.label == "CERTIFICATE" {
                            if let Some(cert) = x509::parse_cert(&p.contents) {
                                trust_store_keys.push(cert.public_key);
                            }
                        }
                    }
                }
            }

            println!("[報告] パッケージ本体をロード中: {}", args.url);
            let data = http_get(&args.url, &trust_store_keys)?;

            println!("[報告] 署名をロード中: {}", sig_url);
            let sig_data = http_get(&sig_url, &trust_store_keys)?;
            if sig_data.len() != 64 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "[エラー] 署名形式が不正です（64バイトである必要があります）。"));
            }

            println!("[報告] データの整合性を確認中 (SHA-512)...");
            let content_hash = sha512::sha512(&data);
            let hash_hex: String = content_hash.iter().map(|b| format!("{:02x}", b)).collect();
            println!("[報告] ハッシュ計算完了。");

            println!("[報告] 署名を検証中 (独自実装 Ed25519)...");
            let pubkey_bytes = fs::read(&args.pubkey)
                .map_err(|_| io::Error::new(io::ErrorKind::NotFound, format!("[エラー] 公開鍵が見つかりません: {}", args.pubkey)))?;
            
            if pubkey_bytes.len() != 32 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "[エラー] 公開鍵のサイズが不正です。"));
            }

            let mut pk = [0u8; 32];
            pk.copy_from_slice(&pubkey_bytes);
            let mut sig = [0u8; 64];
            sig.copy_from_slice(&sig_data);

            if !ed25519::Point::verify(&pk, &data, &sig) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "[警告] 署名検証に失敗しました。不正なバイナリです。"));
            }

            println!("[報告] 検証成功。バイナリを配置します（原子的な配置）... ");
            fs::write(&tmp_path, &data)?;

            let verification_hash = sha512::sha512(&fs::read(&tmp_path)?);
            if content_hash != verification_hash {
                let _ = fs::remove_file(&tmp_path);
                return Err(io::Error::new(io::ErrorKind::WriteZero, "[エラー] 書き込み後のデータ整合性チェックに失敗しました。"));
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&tmp_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&tmp_path, perms)?;
            }

            fs::rename(&tmp_path, &target_path)?;

            // マニフェストデータベースへ追記
            add_to_manifest(&args.name, &args.url, &target_path, &hash_hex)?;

            println!("[報告] インストール完了: {}", target_path);
            Ok(())
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("不明なサブコマンド: {}", args.subcommand))),
    }
}
