//! Task list model. Free of browser APIs so it can be tested with `cargo test`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_LEN: usize = 500;
/// Number of identity colors defined in style.css (`--c0` … `--c5`).
pub const PALETTE_SIZE: usize = 6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub completed: bool,
}

#[derive(Debug, PartialEq)]
pub enum AddError {
    Empty,
}

#[derive(Debug, PartialEq)]
pub struct InvalidSave;

#[derive(Debug, Default)]
pub struct Tasks {
    items: Vec<Task>,
}

impl Tasks {
    /// Reads a saved list. Rejects anything malformed instead of guessing,
    /// so the caller can leave the original save untouched.
    pub fn from_json(json: &str) -> Result<Self, InvalidSave> {
        let items: Vec<Task> = serde_json::from_str(json).map_err(|_| InvalidSave)?;
        let mut ids = HashSet::new();
        let valid = items
            .iter()
            .all(|task| !task.text.trim().is_empty() && ids.insert(task.id.as_str()));
        if valid { Ok(Self { items }) } else { Err(InvalidSave) }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.items).expect("tasks always serialize")
    }

    pub fn add(&mut self, id: String, text: &str) -> Result<(), AddError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(AddError::Empty);
        }
        let text = text.chars().take(MAX_LEN).collect();
        self.items.push(Task { id, text, completed: false });
        Ok(())
    }

    pub fn set_completed(&mut self, id: &str, completed: bool) {
        if let Some(task) = self.items.iter_mut().find(|task| task.id == id) {
            task.completed = completed;
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.items.retain(|task| task.id != id);
    }

    #[cfg(test)]
    pub fn get(&self, id: &str) -> Option<&Task> {
        self.items.iter().find(|task| task.id == id)
    }

    /// The first open task, shown large at the top of the page.
    pub fn next(&self) -> Option<&Task> {
        self.items.iter().find(|task| !task.completed)
    }

    /// Everything except `next()`: open tasks in order, then completed ones.
    pub fn rest(&self) -> Vec<&Task> {
        let next = self.next().map(|task| task.id.as_str());
        let open = self.items.iter().filter(|task| !task.completed && Some(task.id.as_str()) != next);
        let done = self.items.iter().filter(|task| task.completed);
        open.chain(done).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn summary(&self) -> String {
        let total = self.items.len();
        let done = self.items.iter().filter(|task| task.completed).count();
        match (total - done, done) {
            (0, 0) => "Set your own pace.".into(),
            (0, _) => "All done".into(),
            (left, 0) => format!("{left} left"),
            (left, done) => format!("{left} left · {done} done"),
        }
    }
}

/// Stable identity color for a task, so it keeps its color as the list changes.
pub fn color_index(id: &str) -> usize {
    let hash = id.bytes().fold(0u32, |hash, byte| hash.wrapping_mul(31).wrapping_add(byte.into()));
    hash as usize % PALETTE_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(entries: &[(&str, bool)]) -> Tasks {
        let mut tasks = Tasks::default();
        for (text, completed) in entries {
            let id = format!("id-{text}");
            tasks.add(id.clone(), text).unwrap();
            tasks.set_completed(&id, *completed);
        }
        tasks
    }

    #[test]
    fn rejects_blank_tasks_and_trims_text() {
        let mut tasks = Tasks::default();
        assert_eq!(tasks.add("a".into(), "   \t "), Err(AddError::Empty));
        tasks.add("b".into(), "  Çay demle  ").unwrap();
        assert_eq!(tasks.get("b").unwrap().text, "Çay demle");
    }

    #[test]
    fn caps_text_length_by_characters() {
        let mut tasks = Tasks::default();
        tasks.add("a".into(), &"ğ".repeat(MAX_LEN + 20)).unwrap();
        assert_eq!(tasks.get("a").unwrap().text.chars().count(), MAX_LEN);
    }

    #[test]
    fn next_is_first_open_task_and_rest_puts_done_last() {
        let tasks = list(&[("one", true), ("two", false), ("three", false), ("four", true)]);
        assert_eq!(tasks.next().unwrap().text, "two");
        let rest: Vec<_> = tasks.rest().iter().map(|task| task.text.as_str()).collect();
        assert_eq!(rest, ["three", "one", "four"]);
    }

    #[test]
    fn summary_covers_every_state() {
        assert_eq!(list(&[]).summary(), "Set your own pace.");
        assert_eq!(list(&[("a", false), ("b", false)]).summary(), "2 left");
        assert_eq!(list(&[("a", false), ("b", true)]).summary(), "1 left · 1 done");
        assert_eq!(list(&[("a", true)]).summary(), "All done");
    }

    #[test]
    fn round_trips_through_json() {
        let tasks = list(&[("a", true), ("b", false)]);
        let restored = Tasks::from_json(&tasks.to_json()).unwrap();
        assert_eq!(restored.items, tasks.items);
    }

    #[test]
    fn rejects_invalid_saves() {
        for json in [
            "{}",
            "not json",
            r#"[{"id":"a","text":"x"}]"#,
            r#"[{"id":"a","text":"  ","completed":false}]"#,
            r#"[{"id":"a","text":"x","completed":false},{"id":"a","text":"y","completed":true}]"#,
        ] {
            assert_eq!(Tasks::from_json(json).unwrap_err(), InvalidSave, "{json}");
        }
    }

    #[test]
    fn reads_saves_from_the_javascript_version() {
        let json = r#"[{"id":"7f3c","text":"Kitabımdan 10 sayfa oku","completed":false}]"#;
        assert_eq!(Tasks::from_json(json).unwrap().next().unwrap().text, "Kitabımdan 10 sayfa oku");
    }
}
