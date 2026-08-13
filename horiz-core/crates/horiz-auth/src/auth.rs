use std::fs;
use std::io::{self, BufRead};
use crate::sha256::sha256;
use crate::base64::base64_encode;

pub fn hash_password(password: &str, salt: &str) -> String {
    let mut input = Vec::new();
    input.extend_from_slice(salt.as_bytes());
    input.extend_from_slice(password.as_bytes());
    let mut result = sha256(&input);

    // 10,000回のストレッチング
    for _ in 0..10000 {
        result = sha256(&result);
    }

    base64_encode(&result)
}

pub fn verify_login(username: &str, password: &str) -> io::Result<bool> {
    let file = fs::File::open("/etc/shadow")?;
    let reader = io::BufReader::new(file);

    let mut user_found = false;
    let mut is_valid = false;
    let mut target_salt = String::from("dummy_salt_for_timing_mitigation");
    let mut target_hash = String::new();

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 2 { continue; }

        if parts[0] == username {
            let encoded_pwd = parts[1];
            if !encoded_pwd.starts_with("$hz$") { continue; }
            let segments: Vec<&str> = encoded_pwd.split('$').collect();
            if segments.len() < 4 { continue; }

            target_salt = segments[2].to_string();
            target_hash = segments[3].to_string();
            user_found = true;
            break; // ユーザーを見つけたらループを抜ける
        }
    }

    // ユーザーの有無に関わらず、常にハッシュ計算を実行する (定数時間)
    let computed_hash = hash_password(password, &target_salt);

    if user_found {
        if computed_hash.len() == target_hash.len() {
            let mut res = 0u8;
            let a_bytes = computed_hash.as_bytes();
            let b_bytes = target_hash.as_bytes();
            for i in 0..a_bytes.len() {
                res |= a_bytes[i] ^ b_bytes[i];
            }
            is_valid = res == 0;
        }
    }

    Ok(is_valid)
}

pub fn generate_shadow_entry(password: &str, salt: &str) -> String {
    let hash = hash_password(password, salt);
    format!("$hz${}${}", salt, hash)
}

/// セキュアなソルトを生成 (CSPRNG - Zero-Dependency)
pub fn generate_salt() -> io::Result<String> {
    let mut buf = [0u8; 16];
    let mut f = fs::File::open("/dev/urandom")?;
    io::Read::read_exact(&mut f, &mut buf)?;
    Ok(base64_encode(&buf))
}

pub fn change_password(username: &str, old_password: &str, new_password: &str) -> io::Result<bool> {
    if !verify_login(username, old_password)? {
        return Ok(false);
    }

    let shadow_path = "/etc/shadow";
    let tmp_path = "/etc/shadow.tmp";

    let file = fs::File::open(shadow_path)?;
    let reader = io::BufReader::new(file);
    let mut lines = Vec::new();

    let new_salt = generate_salt().unwrap_or_else(|_| "default_salt".to_string());
    let new_entry = generate_shadow_entry(new_password, &new_salt);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(':').collect();
        if !parts.is_empty() && parts[0] == username {
            let mut new_parts = parts.clone();
            new_parts[1] = &new_entry;
            lines.push(new_parts.join(":"));
        } else {
            lines.push(line);
        }
    }

    let mut tmp_file = fs::File::create(tmp_path)?;
    for l in lines {
        use std::io::Write;
        writeln!(tmp_file, "{}", l)?;
    }

    fs::rename(tmp_path, shadow_path)?;
    Ok(true)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256(b"hello");
        assert_eq!(hex::encode(hash), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_base64() {
        assert_eq!(base64_encode(b"any car"), "YW55IGNhcg==");
    }

    #[test]
    fn print_hashes() {
        println!("root: {}", generate_shadow_entry("root", "root_salt"));
        println!("horiz: {}", generate_shadow_entry("horiz", "horiz_salt"));
    }
}

// 簡易的なhexエンコード (テスト用)
mod hex {
    #[allow(dead_code)]
    pub fn encode(data: [u8; 32]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
