//! Navigation for the task view: a stack of open tasks and the data one level renders.

use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::views;

/// Open tasks, oldest first. The last one is on screen.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct ViewStack(pub Vec<Id>);

impl ViewStack {
    /// Replaces the stack with just `id`.
    pub fn open(&mut self, id: Id) {
        self.0 = vec![id];
    }

    /// Replaces the stack with `id` on top of its ancestors, top-level first,
    /// so Back walks up to the parent. The walk follows `parent_id` through
    /// live tasks in `all` and stops at a missing or deleted parent, or at a cycle.
    pub fn open_chain(&mut self, id: Id, all: &[Task]) {
        self.0 = views::chain(&id, all);
    }

    /// Opens a sub-task on top of the current task.
    pub fn push(&mut self, id: Id) {
        self.0.push(id);
    }

    /// Pops one level; popping the last closes the view.
    pub fn back(&mut self) {
        self.0.pop();
    }

    pub fn close(&mut self) {
        self.0.clear();
    }

    /// The task was completed from the view's own circle: same as `back`.
    pub fn completed_here(&mut self) {
        self.back();
    }

    /// Cuts the stack at the first id not in `all`. A deleted task's sub-tasks are
    /// deleted with it, so everything above it goes too.
    pub fn prune(&mut self, all: &[Task]) {
        let keep = self.0.iter().take_while(|id| all.iter().any(|t| &t.id == *id)).count();
        self.0.truncate(keep);
    }

    pub fn top(&self) -> Option<&Id> {
        self.0.last()
    }
}

pub use keel_core::task_view::{detail, TaskDetail};

/// Context shared by the whole app: the stack and the view's own show-completed toggle.
#[derive(Clone, Copy)]
pub struct TaskViewNav {
    pub stack: Signal<ViewStack>,
    pub show_completed: Signal<bool>,
    /// Every task, refreshed with each load (rows open tasks with their ancestors).
    pub all: Signal<Vec<Task>>,
    /// Set when a view should open with its "Add sub-task" editor showing;
    /// the view clears it.
    pub add_sub: Signal<bool>,
    /// Set by a tap on the sheet's scrim or a drag that pulls it down: the
    /// view saves any pending edit, then closes, and clears it.
    pub close_asked: Signal<bool>,
    /// The view's completion circles ignore taps (see [`TaskViewNav::hold_circles`]).
    pub circles_held: Signal<bool>,
}

/// How long the circles stay held after a tap's call returns: longer than
/// the gap between a double tap's two taps.
const CIRCLE_SETTLE_MS: u32 = 400;

pub fn provide_nav() -> TaskViewNav {
    use_context_provider(|| TaskViewNav {
        stack: Signal::new(ViewStack::default()),
        show_completed: Signal::new(false),
        all: Signal::new(Vec::new()),
        add_sub: Signal::new(false),
        close_asked: Signal::new(false),
        circles_held: Signal::new(false),
    })
}

pub fn use_nav() -> TaskViewNav {
    use_context()
}

impl TaskViewNav {
    /// Opens a task fresh; show-completed starts off.
    pub fn open(mut self, id: Id) {
        self.show_completed.set(false);
        self.stack.write().open(id);
    }

    /// Opens a task from a list with its ancestors under it; show-completed starts off.
    pub fn open_chain(mut self, id: Id, all: &[Task]) {
        self.show_completed.set(false);
        self.stack.write().open_chain(id, all);
    }

    /// Opens a task like [`Self::open_chain`], with its "Add sub-task" editor open.
    pub fn open_adding(mut self, id: Id, all: &[Task]) {
        self.open_chain(id, all);
        self.add_sub.set(true);
    }

    pub fn push(mut self, id: Id) {
        self.stack.write().push(id);
    }

    pub fn back(mut self) {
        self.stack.write().back();
    }

    pub fn close(mut self) {
        self.stack.write().close();
    }

    /// Asks the open view to close (it saves a pending edit first).
    pub fn ask_close(mut self) {
        if !self.stack.peek().0.is_empty() {
            self.close_asked.set(true);
        }
    }

    /// Holds the view's completion circles for one tap; false when they're
    /// already held. A double tap's second tap would otherwise complete the
    /// task twice, or land on the circle that took the first one's place
    /// (the parent's after a pop, or the next sub-task's).
    pub fn hold_circles(mut self) -> bool {
        if *self.circles_held.peek() {
            return false;
        }
        self.circles_held.set(true);
        true
    }

    /// Lets the circles take taps again, a moment after the tap's call returned.
    pub async fn release_circles(mut self) {
        crate::completion::sleep(CIRCLE_SETTLE_MS).await;
        self.circles_held.set(false);
    }

    /// The task on top was completed from its own circle.
    pub fn completed_here(mut self) {
        self.stack.write().completed_here();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Id {
        Id::from(s)
    }

    fn task(s: &str, parent: Option<&str>, order: i64) -> Task {
        let mut t = Task::new(s, id("proj"));
        t.id = id(s);
        t.parent_id = parent.map(id);
        t.child_order = order;
        t
    }

    fn stack(ids: &[&str]) -> ViewStack {
        ViewStack(ids.iter().map(|s| id(s)).collect())
    }

    #[test]
    fn open_replaces_stack() {
        let mut s = ViewStack::default();
        s.open(id("A"));
        s.push(id("B"));
        s.open(id("C"));
        assert_eq!(s, stack(&["C"]));
    }

    #[test]
    fn open_chain_replaces_the_stack_with_the_chain() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0), task("G", Some("C"), 0)];
        let mut s = stack(&["X"]);
        s.open_chain(id("G"), &all);
        assert_eq!(s, stack(&["P", "C", "G"]));
    }

    #[test]
    fn back_pops_then_closes() {
        let mut s = stack(&["A", "B"]);
        s.back();
        assert_eq!(s, stack(&["A"]));
        s.back();
        assert_eq!(s, stack(&[]));
        s.back();
        assert_eq!(s, stack(&[]));
    }

    #[test]
    fn complete_here_goes_to_parent() {
        let mut s = stack(&["A", "B"]);
        s.completed_here();
        assert_eq!(s, stack(&["A"]));
        s.completed_here();
        assert_eq!(s, stack(&[]));
    }

    #[test]
    fn prune_cuts_at_deleted() {
        // B and its sub-task C are deleted, so neither is in `all`.
        let all = vec![task("A", None, 0)];
        let mut s = stack(&["A", "B", "C"]);
        s.prune(&all);
        assert_eq!(s, stack(&["A"]));
    }

    #[test]
    fn prune_cuts_even_if_later_ids_exist() {
        let all = vec![task("A", None, 0), task("C", Some("B"), 0)];
        let mut s = stack(&["A", "B", "C"]);
        s.prune(&all);
        assert_eq!(s, stack(&["A"]));
    }

    #[test]
    fn prune_everything_closes() {
        let mut s = stack(&["A"]);
        s.prune(&[]);
        assert_eq!(s, stack(&[]));
    }

    #[test]
    fn completed_elsewhere_stays_open() {
        let mut a = task("A", None, 0);
        a.checked = true;
        let all = vec![a];
        let mut s = stack(&["A"]);
        s.prune(&all);
        assert_eq!(s, stack(&["A"]));
    }
}
