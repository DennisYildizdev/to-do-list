//! Browser side: reads and writes the DOM in index.html and keeps the list in localStorage.

mod tasks;

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, Storage};

use tasks::{AddError, Task, Tasks, color_index};

/// Same key as the JavaScript version, so existing lists carry over.
const STORAGE_KEY: &str = "yapilacaklar.tasks.v1";
const LOAD_WARNING: &str = "Your saved tasks couldn't be read. To protect them, new changes are kept only while this page is open.";
const SAVE_WARNING: &str = "Your tasks couldn't be saved. You can keep working while this page is open, but closing or reloading it may lose recent changes.";
const TRASH_ICON: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v6M14 11v6"/></svg>"#;
/// Tasks longer than this get the smaller headline size.
const LONG_HEADLINE: usize = 80;

struct App {
    doc: Document,
    tasks: Tasks,
    storage: Option<Storage>,
    /// When the saved list was unreadable we never overwrite it.
    unreadable: bool,
    /// What the focus area currently shows, so it only animates when that changes.
    shown: String,
}

/// Where a delete came from, to decide where keyboard focus goes next.
enum Origin {
    Focus,
    List(usize),
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app(f: impl FnOnce(&mut App)) {
    APP.with(|app| {
        if let Some(app) = app.borrow_mut().as_mut() {
            f(app);
        }
    });
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let doc = window.document().ok_or("no document")?;
    let storage = window.local_storage().ok().flatten();
    let (tasks, unreadable) = load(storage.as_ref());

    let mut app = App { doc, tasks, storage, unreadable, shown: String::new() };
    if unreadable {
        app.warn(LOAD_WARNING);
    }
    bind(&app.doc);
    app.render();
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    Ok(())
}

fn load(storage: Option<&Storage>) -> (Tasks, bool) {
    let Some(storage) = storage else {
        return (Tasks::default(), true);
    };
    match storage.get_item(STORAGE_KEY) {
        Ok(None) => (Tasks::default(), false),
        Ok(Some(json)) => match Tasks::from_json(&json) {
            Ok(tasks) => (tasks, false),
            Err(_) => (Tasks::default(), true),
        },
        Err(_) => (Tasks::default(), true),
    }
}

fn listen(doc: &Document, id: &str, event: &str, handler: impl FnMut(Event) + 'static) {
    let target = doc.get_element_by_id(id).expect_throw(id);
    let closure = Closure::<dyn FnMut(Event)>::new(handler);
    target
        .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
        .expect_throw(event);
    // Listeners live as long as the page.
    closure.forget();
}

/// The `data-id` of the list item an event happened in, if any.
fn item_id(event: &Event, selector: &str) -> Option<(String, Element)> {
    let target: Element = event.target()?.dyn_into().ok()?;
    target.closest(selector).ok()??;
    let item = target.closest("li").ok()??;
    Some((item.get_attribute("data-id")?, target))
}

fn bind(doc: &Document) {
    listen(doc, "task-form", "submit", |event| {
        event.prevent_default();
        with_app(App::add);
    });
    listen(doc, "task-input", "input", |_| with_app(App::clear_error));
    listen(doc, "done", "click", |_| with_app(App::complete_next));
    listen(doc, "delete-next", "click", |_| {
        with_app(|app| {
            if let Some(id) = app.tasks.next().map(|task| task.id.clone()) {
                app.delete(&id, Origin::Focus);
            }
        });
    });
    listen(doc, "task-list", "change", |event| {
        if let Some((id, target)) = item_id(&event, "input[type=checkbox]") {
            let checked = target.unchecked_into::<HtmlInputElement>().checked();
            with_app(|app| app.toggle(&id, checked));
        }
    });
    listen(doc, "task-list", "click", |event| {
        if let Some((id, _)) = item_id(&event, "button.delete") {
            with_app(|app| {
                let index = app.tasks.rest().iter().position(|task| task.id == id).unwrap_or(0);
                app.delete(&id, Origin::List(index));
            });
        }
    });
}

fn new_id() -> String {
    // Date + random is enough for a list that lives in one browser,
    // and unlike crypto.randomUUID it also works outside secure contexts.
    let random = (js_sys::Math::random() * 9_007_199_254_740_992.0) as u64;
    format!("{:x}-{random:x}", js_sys::Date::now() as u64)
}

fn set_hidden(element: &Element, hidden: bool) {
    let _ = element.toggle_attribute_with_force("hidden", hidden);
}

fn focus(element: &Element) {
    if let Some(element) = element.dyn_ref::<HtmlElement>() {
        let _ = element.focus();
    }
}

impl App {
    fn el(&self, id: &str) -> Element {
        self.doc.get_element_by_id(id).expect_throw(id)
    }

    fn input(&self) -> HtmlInputElement {
        self.el("task-input").unchecked_into()
    }

    fn warn(&self, message: &str) {
        let warning = self.el("storage-error");
        warning.set_text_content(Some(message));
        set_hidden(&warning, false);
    }

    fn announce(&self, message: &str) {
        self.el("announcement").set_text_content(Some(message));
    }

    fn save(&self) {
        if self.unreadable {
            return;
        }
        let saved = self
            .storage
            .as_ref()
            .is_some_and(|storage| storage.set_item(STORAGE_KEY, &self.tasks.to_json()).is_ok());
        if saved {
            set_hidden(&self.el("storage-error"), true);
        } else {
            self.warn(SAVE_WARNING);
        }
    }

    fn render(&mut self) {
        self.shown = self.render_focus();
        self.render_list();
        self.el("count").set_text_content(Some(&self.tasks.summary()));
    }

    /// Fills the large area at the top and returns a key for what it shows.
    fn render_focus(&self) -> String {
        let eyebrow = self.el("focus-eyebrow");
        let title = self.el("focus-title");
        let note = self.el("focus-note");
        let actions = self.el("focus-actions");

        let shown = match self.tasks.next() {
            Some(task) => {
                eyebrow.set_attribute("style", &format!("--c: var(--c{})", color_index(&task.id))).ok();
                title.set_text_content(Some(&task.text));
                let _ = title.class_list().toggle_with_force("long", task.text.chars().count() > LONG_HEADLINE);
                self.el("delete-next")
                    .set_attribute("aria-label", &format!("Delete “{}”", task.text))
                    .ok();
                task.id.clone()
            }
            None => {
                let (headline, hint, key) = if self.tasks.is_empty() {
                    ("Room for a new step.", "Write your first one below. A small step is enough to begin.", "empty")
                } else {
                    ("All done. Take a break.", "Add another step whenever you’re ready.", "done")
                };
                title.set_text_content(Some(headline));
                let _ = title.class_list().remove_1("long");
                note.set_text_content(Some(hint));
                key.to_owned()
            }
        };
        let has_next = self.tasks.next().is_some();
        set_hidden(&eyebrow, !has_next);
        set_hidden(&actions, !has_next);
        set_hidden(&note, has_next);

        if shown != self.shown {
            // Restart the entrance animation: drop the class, force a reflow, add it back.
            let focus_area = self.el("focus");
            let _ = focus_area.class_list().remove_1("enter");
            let _ = focus_area.unchecked_ref::<HtmlElement>().offset_width();
            let _ = focus_area.class_list().add_1("enter");
        }
        shown
    }

    fn render_list(&self) {
        let list = self.el("task-list");
        list.set_text_content(None);
        let rest = self.tasks.rest();
        for task in &rest {
            let _ = list.append_child(&self.item(task));
        }
        set_hidden(&list, rest.is_empty());
    }

    fn item(&self, task: &Task) -> Element {
        let create = |tag: &str| self.doc.create_element(tag).expect_throw(tag);

        let item = create("li");
        item.set_attribute("data-id", &task.id).ok();
        item.set_attribute("style", &format!("--c: var(--c{})", color_index(&task.id))).ok();
        if task.completed {
            item.set_class_name("completed");
        }

        let label = create("label");
        label.set_class_name("task-label");
        let checkbox: HtmlInputElement = create("input").unchecked_into();
        checkbox.set_type("checkbox");
        checkbox.set_checked(task.completed);
        let text = create("span");
        text.set_class_name("task-text");
        text.set_text_content(Some(&task.text));
        let _ = label.append_child(&checkbox);
        let _ = label.append_child(&text);

        let remove = create("button");
        remove.set_attribute("type", "button").ok();
        remove.set_class_name("delete");
        remove.set_attribute("aria-label", &format!("Delete “{}”", task.text)).ok();
        remove.set_attribute("title", "Delete task").ok();
        remove.set_inner_html(TRASH_ICON);

        let _ = item.append_child(&label);
        let _ = item.append_child(&remove);
        item
    }

    /// The list item for a task, matched by attribute so ids never need CSS escaping.
    fn find_item(&self, id: &str) -> Option<Element> {
        let items = self.el("task-list").children();
        (0..items.length())
            .filter_map(|i| items.item(i))
            .find(|item| item.get_attribute("data-id").as_deref() == Some(id))
    }

    /// Done button while there is a next task, otherwise the input.
    fn focus_main_action(&self) {
        if self.tasks.next().is_some() {
            focus(&self.el("done"));
        } else {
            let _ = self.input().focus();
        }
    }

    fn clear_error(&mut self) {
        self.el("input-error").set_text_content(None);
        let _ = self.input().remove_attribute("aria-invalid");
    }

    fn add(&mut self) {
        let input = self.input();
        if let Err(AddError::Empty) = self.tasks.add(new_id(), &input.value()) {
            self.el("input-error").set_text_content(Some("Write a task to add it."));
            input.set_attribute("aria-invalid", "true").ok();
            let _ = input.focus();
            return;
        }
        self.save();
        self.render();
        input.set_value("");
        self.clear_error();
        let _ = input.focus();
        self.announce("Task added.");
    }

    fn complete_next(&mut self) {
        let Some(id) = self.tasks.next().map(|task| task.id.clone()) else { return };
        self.tasks.set_completed(&id, true);
        self.after_change();
        self.focus_main_action();
        self.announce("Task completed.");
    }

    fn toggle(&mut self, id: &str, completed: bool) {
        self.tasks.set_completed(id, completed);
        self.after_change();
        // Reopening a task can move it into the focus area; follow it there.
        match self.find_item(id).and_then(|item| item.query_selector("input").ok().flatten()) {
            Some(checkbox) => focus(&checkbox),
            None => self.focus_main_action(),
        }
        self.announce(if completed { "Task completed." } else { "Task moved back to your list." });
    }

    fn delete(&mut self, id: &str, origin: Origin) {
        self.tasks.remove(id);
        self.after_change();
        let neighbour = match origin {
            Origin::List(index) => {
                let items = self.el("task-list").children();
                let last = items.length().checked_sub(1);
                last.and_then(|last| items.item((index as u32).min(last)))
            }
            Origin::Focus => None,
        };
        match neighbour.and_then(|item| item.query_selector(".delete").ok().flatten()) {
            Some(button) => focus(&button),
            None => self.focus_main_action(),
        }
        self.announce("Task deleted.");
    }

    fn after_change(&mut self) {
        self.save();
        self.render();
    }
}
