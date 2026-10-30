//! `keel-core` for the web UI (`web/src/core`), over wasm-bindgen.
//!
//! Each export takes and returns the ts-rs binding shapes (`web/src/bindings`):
//! inputs are deserialized with serde-wasm-bindgen, outputs serialized with
//! `Serializer::json_compatible()` so `Option` is `null`, maps are plain
//! objects and `i64` is a `number`, exactly as over IPC. The
//! `unchecked_*_type` attributes name those bindings, so the generated
//! `.d.ts` has no `any`. The paths are relative to `web/src/core/pkg/`.
//!
//! Only data rules live here, and only as thin calls into `keel_core`: date
//! strings are parsed here, everything else is `keel_core`'s.

use chrono::{NaiveDate, NaiveTime};
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::views::{self, CardMode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Parses a `YYYY-MM-DD` date, or the date part of a `YYYY-MM-DDTHH:MM…`
/// datetime (a `Due.date`).
fn parse_date(s: &str) -> Result<NaiveDate, String> {
    let date = match s.get(10..11) {
        None if s.len() == 10 => s,
        Some("T") => &s[..10],
        _ => return Err(format!("invalid date \"{s}\": expected YYYY-MM-DD")),
    };
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| format!("invalid date \"{s}\": expected YYYY-MM-DD"))
}

/// Parses an `HH:MM` or `HH:MM:SS` time of day.
fn parse_time(s: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
        .map_err(|_| format!("invalid time \"{s}\": expected HH:MM or HH:MM:SS"))
}

fn input<T: DeserializeOwned>(name: &str, value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&format!("invalid {name}: {e}")))
}

fn output<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&format!("serialize: {e}")))
}

/// `views::due_label` for two date strings (`due` may be a datetime).
#[wasm_bindgen(js_name = dueLabel)]
pub fn due_label(due: &str, today: &str) -> Result<String, JsError> {
    let due = parse_date(due).map_err(|e| JsError::new(&e))?;
    let today = parse_date(today).map_err(|e| JsError::new(&e))?;
    Ok(views::due_label(due, today))
}

/// `views::time_label` for an `HH:MM[:SS]` string.
#[wasm_bindgen(js_name = timeLabel)]
pub fn time_label(time: &str) -> Result<String, JsError> {
    Ok(views::time_label(parse_time(time).map_err(|e| JsError::new(&e))?))
}

/// `views::progress`: a parent's sub-task counts, `null` when it has none.
#[wasm_bindgen(js_name = progress, unchecked_return_type = "import('../../bindings').Progress | null")]
pub fn progress(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Id")] parent: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] all: JsValue,
) -> Result<JsValue, JsError> {
    let parent: Id = input("parent", parent)?;
    let all: Vec<Task> = input("all", all)?;
    output(&views::progress(&parent, &all))
}

/// `views::cards`: groups a view's matched tasks into cards.
#[wasm_bindgen(js_name = cards, unchecked_return_type = "import('../../bindings').Card[]")]
pub fn cards(
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] matched: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').Task[]")] all: JsValue,
    #[wasm_bindgen(unchecked_param_type = "import('../../bindings').CardMode")] mode: JsValue,
) -> Result<JsValue, JsError> {
    let matched: Vec<Task> = input("matched", matched)?;
    let all: Vec<Task> = input("all", all)?;
    let mode: CardMode = input("mode", mode)?;
    output(&views::cards(matched, &all, mode))
}

#[cfg(test)]
mod tests {
    use super::{parse_date, parse_time};
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn parses_a_date() {
        assert_eq!(parse_date("2026-10-30"), Ok(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()));
    }

    #[test]
    fn parses_the_date_of_a_datetime() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        assert_eq!(parse_date("2026-10-30T17:00:00"), Ok(d));
        assert_eq!(parse_date("2026-10-30T17:00:00Z"), Ok(d));
    }

    #[test]
    fn rejects_malformed_dates() {
        for bad in ["", "30/10/2026", "2026-10-3", "2026-13-01", "2026-10-30 17:00", "2026-10-30x"] {
            assert_eq!(parse_date(bad), Err(format!("invalid date \"{bad}\": expected YYYY-MM-DD")), "{bad}");
        }
    }

    #[test]
    fn parses_times() {
        let t = NaiveTime::from_hms_opt(17, 30, 0).unwrap();
        assert_eq!(parse_time("17:30"), Ok(t));
        assert_eq!(parse_time("17:30:00"), Ok(t));
    }

    #[test]
    fn rejects_malformed_times() {
        assert_eq!(parse_time("5pm"), Err("invalid time \"5pm\": expected HH:MM or HH:MM:SS".to_string()));
        assert_eq!(parse_time("25:00"), Err("invalid time \"25:00\": expected HH:MM or HH:MM:SS".to_string()));
    }
}
