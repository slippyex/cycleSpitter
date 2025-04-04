use once_cell::sync::Lazy;
use regex::Regex;

pub static REG_NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|\s)\(\s*(\d+)\s*\)").unwrap()
});
