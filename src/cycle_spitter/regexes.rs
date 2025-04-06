use once_cell::sync::Lazy;
use regex::Regex;

pub static REG_NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|\s)\(\s*(\d+)\s*\)").unwrap()
});

pub static REG_LABEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\s|\.*[a-zA-Z_][a-zA-Z0-9_]*:)(.*)$").unwrap()
});