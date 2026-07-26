pub(crate) fn osc52_copy_sequence(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()))
}

#[allow(dead_code)]
pub(crate) const fn osc52_read_query() -> &'static str {
    "\x1b]52;c;?\x07"
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub(super) fn base64_decode(encoded: &[u8], max_bytes: usize) -> Option<Vec<u8>> {
    let remainder = encoded.len() % 4;
    if remainder == 1 {
        return None;
    }

    let padding = encoded
        .iter()
        .rev()
        .take_while(|&&byte| byte == b'=')
        .count();
    if padding > 2
        || (padding != 0 && remainder != 0)
        || encoded[..encoded.len().saturating_sub(padding)].contains(&b'=')
    {
        return None;
    }

    let unpadded_len = encoded.len() - padding;
    let full_len = unpadded_len / 4 * 4;
    let mut decoded = Vec::new();

    for chunk in encoded[..full_len].chunks_exact(4) {
        let a = base64_value(chunk[0])?;
        let b = base64_value(chunk[1])?;
        let c = base64_value(chunk[2])?;
        let d = base64_value(chunk[3])?;
        push_decoded(&mut decoded, (a << 2) | (b >> 4), max_bytes)?;
        push_decoded(&mut decoded, (b << 4) | (c >> 2), max_bytes)?;
        push_decoded(&mut decoded, (c << 6) | d, max_bytes)?;
    }

    let tail = &encoded[full_len..unpadded_len];
    match tail {
        [] if padding == 0 => {}
        [a, b] if padding == 0 || padding == 2 => {
            let a = base64_value(*a)?;
            let b = base64_value(*b)?;
            if b & 0x0f != 0 {
                return None;
            }
            push_decoded(&mut decoded, (a << 2) | (b >> 4), max_bytes)?;
        }
        [a, b, c] if padding == 0 || padding == 1 => {
            let a = base64_value(*a)?;
            let b = base64_value(*b)?;
            let c = base64_value(*c)?;
            if c & 0x03 != 0 {
                return None;
            }
            push_decoded(&mut decoded, (a << 2) | (b >> 4), max_bytes)?;
            push_decoded(&mut decoded, (b << 4) | (c >> 2), max_bytes)?;
        }
        _ => return None,
    }

    Some(decoded)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn push_decoded(decoded: &mut Vec<u8>, byte: u8, max_bytes: usize) -> Option<()> {
    if decoded.len() >= max_bytes {
        return None;
    }
    decoded.push(byte);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_read_query_is_exact() {
        assert_eq!(osc52_read_query(), "\x1b]52;c;?\x07");
    }

    #[test]
    fn base64_hardcoded_vectors_encode_and_decode() {
        for (plain, encoded) in [
            (b"".as_slice(), ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foobar", "Zm9vYmFy"),
            (b"\xff", "/w=="),
        ] {
            assert_eq!(base64_encode(plain), encoded);
            assert_eq!(
                base64_decode(encoded.as_bytes(), plain.len()),
                Some(plain.to_vec())
            );
        }
        assert_eq!(base64_decode(b"Zg", 1), Some(b"f".to_vec()));
        assert_eq!(base64_decode(b"Zm8", 2), Some(b"fo".to_vec()));
    }

    #[test]
    fn base64_decode_enforces_exact_decoded_cap() {
        assert_eq!(base64_decode(b"Zm9v", 3), Some(b"foo".to_vec()));
        assert_eq!(base64_decode(b"Zm9vYg==", 3), None);
        assert_eq!(base64_decode(b"", 0), Some(Vec::new()));
        assert_eq!(base64_decode(b"AA==", 0), None);
    }

    #[test]
    fn base64_decode_rejects_malformed_input() {
        for encoded in [
            b"Zm$v".as_slice(),
            b"Z",
            b"=m9v",
            b"Z=9v",
            b"Zm=v",
            b"Zg===",
            b"Zg=",
            b"Zh==",
            b"Zm9=",
            b"Zh",
            b"Zm9",
            b"Zm 9v",
            b"Zm9v\n",
        ] {
            assert_eq!(base64_decode(encoded, 64), None, "{encoded:?}");
        }
    }

    #[test]
    fn base64_round_trips_every_byte_value() {
        let bytes: Vec<u8> = (0..=u8::MAX).collect();
        let encoded = base64_encode(&bytes);
        assert_eq!(base64_decode(encoded.as_bytes(), bytes.len()), Some(bytes));
    }
}
