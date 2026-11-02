use bytes::BufMut;

pub fn parse_i64(src: &[u8]) -> Option<i64> {
    match src.strip_prefix(b"-") {
        Some(digits) => 0i64.checked_sub_unsigned(parse_u64(digits)?),
        None => i64::try_from(parse_u64(src)?).ok(),
    }
}

pub fn parse_usize(src: &[u8]) -> Option<usize> {
    usize::try_from(parse_u64(src)?).ok()
}

pub fn put_i64<B: BufMut>(dst: &mut B, n: i64) {
    if n < 0 {
        dst.put_u8(b'-');
    }
    put_u64(dst, n.unsigned_abs());
}

pub fn put_usize<B: BufMut>(dst: &mut B, n: usize) {
    put_u64(dst, n as u64);
}

fn parse_u64(src: &[u8]) -> Option<u64> {
    if src.is_empty() {
        return None;
    }

    let mut n: u64 = 0;
    for &b in src {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    Some(n)
}

fn put_u64<B: BufMut>(dst: &mut B, mut n: u64) {
    // u64::MAX has 20 digits.
    let mut buf = [0u8; 20];
    let mut i = buf.len();

    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }

    dst.put_slice(&buf[i..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_i64_valid() {
        assert_eq!(parse_i64(b"0"), Some(0));
        assert_eq!(parse_i64(b"-0"), Some(0));
        assert_eq!(parse_i64(b"427"), Some(427));
        assert_eq!(parse_i64(b"-427"), Some(-427));
        assert_eq!(parse_i64(b"9223372036854775807"), Some(i64::MAX));
        assert_eq!(parse_i64(b"-9223372036854775808"), Some(i64::MIN));
    }

    #[test]
    fn parse_i64_invalid() {
        assert_eq!(parse_i64(b""), None);
        assert_eq!(parse_i64(b"-"), None);
        assert_eq!(parse_i64(b"+5"), None);
        assert_eq!(parse_i64(b"--5"), None);
        assert_eq!(parse_i64(b"12x"), None);
        assert_eq!(parse_i64(b" 1"), None);
        assert_eq!(parse_i64(b"9223372036854775808"), None);
        assert_eq!(parse_i64(b"-9223372036854775809"), None);
        assert_eq!(parse_i64(b"99999999999999999999999"), None);
    }

    #[test]
    fn parse_usize_cases() {
        assert_eq!(parse_usize(b"0"), Some(0));
        assert_eq!(parse_usize(b"5"), Some(5));
        assert_eq!(parse_usize(b"-1"), None);
        assert_eq!(parse_usize(b"+1"), None);
        assert_eq!(parse_usize(b""), None);
    }

    #[test]
    fn put_i64_matches_std() {
        for n in [0, 7, -7, 9, 10, -10, 1000, i64::MAX, i64::MIN] {
            let mut buf = Vec::new();
            put_i64(&mut buf, n);
            assert_eq!(buf, n.to_string().as_bytes());
        }
    }

    #[test]
    fn put_usize_matches_std() {
        for n in [0, 1, 9, 10, 4096, usize::MAX] {
            let mut buf = Vec::new();
            put_usize(&mut buf, n);
            assert_eq!(buf, n.to_string().as_bytes());
        }
    }

    #[test]
    fn i64_round_trips() {
        for n in [0, 1, -1, 427, -427, i64::MAX, i64::MIN] {
            let mut buf = Vec::new();
            put_i64(&mut buf, n);
            assert_eq!(parse_i64(&buf), Some(n));
        }
    }
}
