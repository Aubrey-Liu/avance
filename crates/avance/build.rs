use version_check as rustc;

fn main() {
    if rustc::is_min_version("1.70.0").unwrap_or(false) {
        println!(r#"cargo:rustc-cfg=has_std_once_cell="true""#)
    } else {
        println!(r#"cargo:rustc-cfg=has_std_once_cell="false""#)
    }
}
