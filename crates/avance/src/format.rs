pub fn format_time(seconds: u64) -> String {
    let m = seconds / 60 % 60;
    let s = seconds % 60;
    match seconds / 3600 {
        0 => format!("{:02}:{:02}", m, s),
        h => format!("{:02}:{:02}:{:02}", h, m, s),
    }
}

pub fn format_sizeof(num: u64) -> String {
    let mut num = num as f64;
    for unit in ["", "k", "M", "G", "T", "P", "E", "Z"] {
        if num < 999.5 {
            if num < 99.95 {
                if num < 9.995 {
                    return format!("{:.2}{}", num, unit);
                }
                return format!("{:.1}{}", num, unit);
            }
            return format!("{:.0}{}", num, unit);
        }
        num /= 1000.0;
    }

    format!("{:.1}Y", num)
}

#[cfg(test)]
mod tests {
    #[test]
    fn format_time() {
        assert_eq!(super::format_time(45), "00:45");
        assert_eq!(super::format_time(30 * 60), "30:00");
        assert_eq!(super::format_time(12 * 60 * 60), "12:00:00");
    }

    #[test]
    fn format_sizeof() {
        assert_eq!(super::format_sizeof(10), "10.0");
        assert_eq!(super::format_sizeof(1_234), "1.23k");
        assert_eq!(super::format_sizeof(12_345), "12.3k");
        assert_eq!(super::format_sizeof(1_234_000), "1.23M");
        assert_eq!(super::format_sizeof(999_000_000), "999M");
        assert_eq!(super::format_sizeof(999_999_000), "1.00G");
    }
}
