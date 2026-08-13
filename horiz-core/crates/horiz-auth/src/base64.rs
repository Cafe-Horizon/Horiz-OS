const BASE64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            3 => [chunk[0], chunk[1], chunk[2]],
            2 => [chunk[0], chunk[1], 0],
            1 => [chunk[0], 0, 0],
            _ => unreachable!(),
        };

        let i0 = (b[0] >> 2) as usize;
        let i1 = (((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize;
        let i2 = (((b[1] & 0x0f) << 2) | (b[2] >> 6)) as usize;
        let i3 = (b[2] & 0x3f) as usize;

        result.push(BASE64_ALPHABET[i0] as char);
        result.push(BASE64_ALPHABET[i1] as char);
        if chunk.len() >= 2 {
            result.push(BASE64_ALPHABET[i2] as char);
        } else {
            result.push('=');
        }
        if chunk.len() >= 3 {
            result.push(BASE64_ALPHABET[i3] as char);
        } else {
            result.push('=');
        }
    }
    result
}
