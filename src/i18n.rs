use std::sync::OnceLock;

use chrono::Weekday;
use dioxus::prelude::*;
use serde_json::Value;

pub static LANG: GlobalSignal<String> = Signal::global(|| "en".to_string());

fn parse(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn map(lang: &str) -> &'static Value {
    static EN: OnceLock<Value> = OnceLock::new();
    static FR: OnceLock<Value> = OnceLock::new();
    static AR: OnceLock<Value> = OnceLock::new();
    match lang {
        "fr" => FR.get_or_init(|| parse(include_str!("i18n/fr.json"))),
        "ar" => AR.get_or_init(|| parse(include_str!("i18n/ar.json"))),
        _ => EN.get_or_init(|| parse(include_str!("i18n/en.json"))),
    }
}

fn lookup(lang: &str, key: &str) -> Option<String> {
    map(lang)
        .get(key)
        .or_else(|| map("en").get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    args.iter()
        .fold(template.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

pub fn normalize(code: &str) -> &'static str {
    match code {
        "fr" => "fr",
        "ar" => "ar",
        _ => "en",
    }
}

pub fn lang() -> String {
    LANG().clone()
}

pub fn set_language(code: &str) {
    let mut signal = LANG.signal();
    signal.set(normalize(code).to_string());
}

pub fn direction() -> &'static str {
    if LANG() == "ar" {
        "rtl"
    } else {
        "ltr"
    }
}

pub fn t(key: &str) -> String {
    let current = LANG();
    lookup(&current, key).unwrap_or_else(|| key.to_string())
}

pub fn ta(key: &str, args: &[(&str, &str)]) -> String {
    interpolate(&t(key), args)
}

pub fn tn(key: &str, count: usize) -> String {
    let current = LANG();
    let singular = count == 1 || (count == 0 && current == "fr");
    let suffix = if singular { "_one" } else { "_other" };
    let template = lookup(&current, &format!("{key}{suffix}"))
        .or_else(|| lookup(&current, key))
        .unwrap_or_else(|| key.to_string());
    interpolate(&template, &[("count", &count.to_string())])
}

pub fn weekday_long(weekday: Weekday) -> String {
    t(&format!("common.weekday.{}", weekday_key(weekday)))
}

pub fn weekday_short(weekday: Weekday) -> String {
    t(&format!("common.weekday.{}Short", weekday_key(weekday)))
}

pub fn month_short(month: u32) -> String {
    t(&format!("common.month.{}", month_key(month)))
}

const fn weekday_key(weekday: Weekday) -> &'static str {
    use chrono::Weekday::{Fri, Mon, Sat, Sun, Thu, Tue, Wed};
    match weekday {
        Mon => "monday",
        Tue => "tuesday",
        Wed => "wednesday",
        Thu => "thursday",
        Fri => "friday",
        Sat => "saturday",
        Sun => "sunday",
    }
}

const fn month_key(month: u32) -> &'static str {
    match month {
        1 => "jan",
        2 => "feb",
        3 => "mar",
        4 => "apr",
        5 => "may",
        6 => "jun",
        7 => "jul",
        8 => "aug",
        9 => "sep",
        10 => "oct",
        11 => "nov",
        _ => "dec",
    }
}
