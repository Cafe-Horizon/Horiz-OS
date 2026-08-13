use std::fs;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

/// 東京時間 (JST: UTC+9) または /etc/localtime, /etc/timezone, TZ 環境変数に従い日時を出力
pub fn date() -> io::Result<()> {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let now_secs = duration.as_secs() as i64;
    let (offset_secs, tz_label) = get_timezone_info(now_secs);
    let secs = now_secs + offset_secs;
    let (year, month, day, hour, min, sec, wday) = seconds_to_datetime(secs);

    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month_str = month_names.get((month.saturating_sub(1)) as usize).unwrap_or(&"Jan");

    println!("{} {} {:02} {:02}:{:02}:{:02} {} {}", wday, month_str, day, hour, min, sec, tz_label, year);
    Ok(())
}

pub fn get_timezone_info(now_timestamp: i64) -> (i64, String) {
    if let Ok(tz_env) = std::env::var("TZ") {
        let tz_env = tz_env.trim();
        if !tz_env.is_empty() {
            if tz_env.starts_with('/') || tz_env.starts_with("./") || tz_env.contains('/') || tz_env.contains('\\') {
                if let Ok(bytes) = fs::read(tz_env) {
                    if let Some(res) = parse_tzif(&bytes, now_timestamp) {
                        return res;
                    }
                }
            }
            return parse_timezone_spec(tz_env);
        }
    }

    if let Ok(bytes) = fs::read("/etc/localtime") {
        if let Some(res) = parse_tzif(&bytes, now_timestamp) {
            return res;
        }
    }

    if let Ok(content) = fs::read_to_string("/etc/timezone") {
        let spec = content.trim();
        if !spec.is_empty() {
            return parse_timezone_spec(spec);
        }
    }

    (9 * 3600, "JST".to_string())
}

/// RFC 8536 準拠の TZif バイナリデータを解析し (UTCオフセット(秒), 略称) を返す (Zero-Dependency)
pub fn parse_tzif(data: &[u8], timestamp: i64) -> Option<(i64, String)> {
    if data.len() < 44 { return None; }
    if &data[0..4] != b"TZif" { return None; }

    let version = data[4];
    
    let _ttisutcnt = u32::from_be_bytes(data[20..24].try_into().ok()?) as usize;
    let _ttisstdcnt = u32::from_be_bytes(data[24..28].try_into().ok()?) as usize;
    let leapcnt = u32::from_be_bytes(data[28..32].try_into().ok()?) as usize;
    let timecnt = u32::from_be_bytes(data[32..36].try_into().ok()?) as usize;
    let typecnt = u32::from_be_bytes(data[36..40].try_into().ok()?) as usize;
    let charcnt = u32::from_be_bytes(data[40..44].try_into().ok()?) as usize;

    if typecnt == 0 { return None; }

    let cursor = 44;
    let v1_trans_times_len = timecnt * 4;
    let v1_trans_types_len = timecnt;
    let v1_ttinfo_len = typecnt * 6;
    let v1_abbrev_len = charcnt;

    if version == b'2' || version == b'3' || version == b'4' {
        let v1_total_data_len = v1_trans_times_len + v1_trans_types_len + v1_ttinfo_len + v1_abbrev_len + (leapcnt * 8) + _ttisstdcnt + _ttisutcnt;
        let v2_header_offset = 44 + v1_total_data_len;
        if data.len() >= v2_header_offset + 44 && &data[v2_header_offset..v2_header_offset + 4] == b"TZif" {
            let v2_h = &data[v2_header_offset..];
            let v2_timecnt = u32::from_be_bytes(v2_h[32..36].try_into().ok()?) as usize;
            let v2_typecnt = u32::from_be_bytes(v2_h[36..40].try_into().ok()?) as usize;
            let v2_charcnt = u32::from_be_bytes(v2_h[40..44].try_into().ok()?) as usize;

            let c64 = v2_header_offset + 44;
            let times_64_end = c64 + v2_timecnt * 8;
            let types_64_end = times_64_end + v2_timecnt;
            let ttinfo_64_end = types_64_end + v2_typecnt * 6;
            let abbrev_64_end = ttinfo_64_end + v2_charcnt;

            if data.len() >= abbrev_64_end && v2_typecnt > 0 {
                let times_data = &data[c64..times_64_end];
                let types_data = &data[times_64_end..types_64_end];
                let ttinfo_data = &data[types_64_end..ttinfo_64_end];
                let abbrev_data = &data[ttinfo_64_end..abbrev_64_end];

                return parse_tz_from_tables(times_data, types_data, ttinfo_data, abbrev_data, v2_timecnt, v2_typecnt, timestamp, true);
            }
        }
    }

    let times_32_end = cursor + v1_trans_times_len;
    let types_32_end = times_32_end + v1_trans_types_len;
    let ttinfo_32_end = types_32_end + v1_ttinfo_len;
    let abbrev_32_end = ttinfo_32_end + v1_abbrev_len;

    if data.len() < abbrev_32_end { return None; }

    let times_data = &data[cursor..times_32_end];
    let types_data = &data[times_32_end..types_32_end];
    let ttinfo_data = &data[types_32_end..ttinfo_32_end];
    let abbrev_data = &data[ttinfo_32_end..abbrev_32_end];

    parse_tz_from_tables(times_data, types_data, ttinfo_data, abbrev_data, timecnt, typecnt, timestamp, false)
}

fn parse_tz_from_tables(
    times_data: &[u8],
    types_data: &[u8],
    ttinfo_data: &[u8],
    abbrev_data: &[u8],
    timecnt: usize,
    _typecnt: usize,
    timestamp: i64,
    is_64bit: bool,
) -> Option<(i64, String)> {
    let selected_type_idx = if timecnt == 0 {
        0
    } else {
        let mut idx = None;
        for i in (0..timecnt).rev() {
            let t = if is_64bit {
                i64::from_be_bytes(times_data[i * 8..(i + 1) * 8].try_into().ok()?)
            } else {
                i32::from_be_bytes(times_data[i * 4..(i + 1) * 4].try_into().ok()?) as i64
            };

            if timestamp >= t {
                idx = Some(i);
                break;
            }
        }

        if let Some(i) = idx {
            types_data[i]
        } else {
            0
        }
    };

    let ttinfo_offset = (selected_type_idx as usize) * 6;
    if ttinfo_offset + 6 > ttinfo_data.len() {
        return None;
    }

    let ttinfo = &ttinfo_data[ttinfo_offset..ttinfo_offset + 6];
    let utoff = i32::from_be_bytes(ttinfo[0..4].try_into().ok()?) as i64;
    let desigidx = ttinfo[5] as usize;

    let abbrev_str = if desigidx < abbrev_data.len() {
        let sub = &abbrev_data[desigidx..];
        let end = sub.iter().position(|&b| b == 0).unwrap_or(sub.len());
        String::from_utf8_lossy(&sub[..end]).to_string()
    } else {
        "UNK".to_string()
    };

    Some((utoff, abbrev_str))
}

pub fn parse_timezone_spec(spec: &str) -> (i64, String) {
    let spec = spec.trim();
    match spec {
        "UTC" | "GMT" | "Z" => (0, "UTC".to_string()),
        "JST" => (9 * 3600, "JST".to_string()),
        _ => {
            if let Some((offset_secs, label)) = parse_offset_str(spec) {
                (offset_secs, label)
            } else {
                (9 * 3600, "JST".to_string())
            }
        }
    }
}

fn parse_offset_str(spec: &str) -> Option<(i64, String)> {
    let s = spec.trim();
    if s.is_empty() { return None; }
    let sign = match s.chars().next()? {
        '+' => 1i64,
        '-' => -1i64,
        _ => return None,
    };
    let rest = &s[1..];
    let parts: Vec<&str> = rest.split(':').collect();
    let (hours, mins) = if parts.len() == 2 {
        let h: i64 = parts[0].parse().ok()?;
        let m: i64 = parts[1].parse().ok()?;
        (h, m)
    } else if rest.len() == 4 {
        let h: i64 = rest[..2].parse().ok()?;
        let m: i64 = rest[2..].parse().ok()?;
        (h, m)
    } else {
        return None;
    };
    let total_secs = sign * (hours * 3600 + mins * 60);
    let label = if sign >= 0 {
        format!("+{:02}{:02}", hours, mins)
    } else {
        format!("-{:02}{:02}", hours, mins)
    };
    Some((total_secs, label))
}

fn seconds_to_datetime(ts: i64) -> (i32, u32, u32, u32, u32, u32, &'static str) {
    let sec = (ts.rem_euclid(60)) as u32;
    let minutes = ts.div_euclid(60);
    let min = (minutes.rem_euclid(60)) as u32;
    let hours = minutes.div_euclid(60);
    let hour = (hours.rem_euclid(24)) as u32;
    let days = hours.div_euclid(24);

    let days_of_week = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let wday_idx = ((days + 4).rem_euclid(7)) as usize;
    let wday = days_of_week[wday_idx];

    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64 + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + if m <= 2 { 1 } else { 0 };

    (year, m, d, hour, min, sec, wday)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_tzif_bytes(offset_secs: i64, abbrev: &str) -> Vec<u8> {
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(b"TZif\0");
        buf.extend_from_slice(&[0u8; 15]);
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(&1u32.to_be_bytes());
        buf.extend_from_slice(&1u32.to_be_bytes());
        let charcnt = (abbrev.len() + 1) as u32;
        buf.extend_from_slice(&charcnt.to_be_bytes());

        buf.extend_from_slice(&0i32.to_be_bytes());
        buf.push(0);
        buf.extend_from_slice(&(offset_secs as i32).to_be_bytes());
        buf.push(0);
        buf.push(0);

        buf.extend_from_slice(abbrev.as_bytes());
        buf.push(0);

        buf
    }

    #[test]
    fn test_seconds_to_datetime_epoch() {
        let (y, m, d, h, min, s, wday) = seconds_to_datetime(9 * 3600);
        assert_eq!((y, m, d, h, min, s, wday), (1970, 1, 1, 9, 0, 0, "Thu"));
    }

    #[test]
    fn test_parse_timezone_spec() {
        assert_eq!(parse_timezone_spec("UTC"), (0, "UTC".to_string()));
        assert_eq!(parse_timezone_spec("+09:00"), (9 * 3600, "+0900".to_string()));
        assert_eq!(parse_timezone_spec("-05:00"), (-5 * 3600, "-0500".to_string()));
    }

    #[test]
    fn test_parse_tzif_jst() {
        let tzif_bytes = generate_tzif_bytes(32400, "JST");
        let res = parse_tzif(&tzif_bytes, 1700000000);
        assert_eq!(res, Some((32400, "JST".to_string())));
    }
}
