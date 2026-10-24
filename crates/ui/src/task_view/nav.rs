//! Navigation for the task view: a stack of open tasks and the data one level renders.

use dioxus::prelude::*;
use keel_core::ids::Id;
use keel_core::model::Task;
use keel_core::views::{self, CardRow, Progress};

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
        let mut chain = vec![id];
        loop {
            let cur = chain.last().expect("chain starts non-empty");
            let parent = all
                .iter()
                .find(|t| &t.id == cur)
                .and_then(|t| t.parent_id.as_ref())
                .and_then(|p| all.iter().find(|t| !t.is_deleted && &t.id == p));
            match parent {
                Some(p) if !chain.contains(&p.id) => chain.push(p.id.clone()),
                _ => break,
            }
        }
        chain.reverse();
        self.0 = chain;
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

/// Everything one level of the view renders.
#[derive(Clone, PartialEq, Debug)]
pub struct TaskDetail {
    pub task: Task,
    pub parent: Option<Task>,
    pub progress: Option<Progress>,
    /// Direct sub-tasks in child order; checked ones only when `show_completed`.
    pub visible: Vec<CardRow>,
}

pub fn detail(id: &Id, all: &[Task], show_completed: bool) -> Option<TaskDetail> {
    let task = all.iter().find(|t| &t.id == id)?.clone();
    let parent = task.parent_id.as_ref().and_then(|p| all.iter().find(|t| &t.id == p)).cloned();
    let visible = views::children(id, all)
        .into_iter()
        .filter(|c| show_completed || !c.checked)
        .map(|c| CardRow { task: c.clone(), progress: views::progress(&c.id, all) })
        .collect();
    Some(TaskDetail { progress: views::progress(id, all), task, parent, visible })
}

/// Context shared by the whole app: the stack and the view's own show-completed toggle.
#[derive(Clone, Copy)]
pub struct TaskViewNav {
    pub stack: Signal<ViewStack>,
    pub show_completed: Signal<bool>,
    /// Every task, refreshed with each load (rows open tasks with their ancestors).
    pub all: Signal<Vec<Task>>,
}

pub fn provide_nav() -> TaskViewNav {
    use_context_provider(|| TaskViewNav {
        stack: Signal::new(ViewStack::default()),
        show_completed: Signal::new(false),
        all: Signal::new(Vec::new()),
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

    pub fn push(mut self, id: Id) {
        self.stack.write().push(id);
    }

    pub fn back(mut self) {
        self.stack.write().back();
    }

    pub fn close(mut self) {
        self.stack.write().close();
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
    fn open_chain_builds_ancestors() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0), task("G", Some("C"), 0)];
        let mut s = stack(&["X"]);
        s.open_chain(id("G"), &all);
        assert_eq!(s, stack(&["P", "C", "G"]));
    }

    #[test]
    fn open_chain_top_level_is_single() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0)];
        let mut s = ViewStack::default();
        s.open_chain(id("P"), &all);
        assert_eq!(s, stack(&["P"]));
    }

    #[test]
    fn open_chain_stops_at_missing_parent() {
        // B is gone (or deleted); C's chain starts at C.
        let mut b = task("B", Some("A"), 0);
        b.is_deleted = true;
        let all = vec![task("A", None, 0), b, task("C", Some("B"), 0), task("D", Some("Z"), 0)];
        let mut s = ViewStack::default();
        s.open_chain(id("C"), &all);
        assert_eq!(s, stack(&["C"]));
        s.open_chain(id("D"), &all);
        assert_eq!(s, stack(&["D"]));
    }

    #[test]
    fn open_chain_cycle_terminates() {
        let all = vec![task("A", Some("B"), 0), task("B", Some("A"), 0)];
        let mut s = ViewStack::default();
        s.open_chain(id("A"), &all);
        assert_eq!(s.top(), Some(&id("A")));
        assert!(s.0.len() <= 2);
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
        assert!(detail(&id("A"), &all, false).unwrap().task.checked);
    }

    #[test]
    fn detail_hides_completed_unless_shown() {
        let mut c = task("C", Some("P"), 0);
        c.checked = true;
        let all = vec![task("P", None, 0), task("S", Some("P"), 1), c];
        let hidden = detail(&id("P"), &all, false).unwrap();
        let ids: Vec<_> = hidden.visible.iter().map(|r| r.task.id.clone()).collect();
        assert_eq!(ids, vec![id("S")]);
        let shown = detail(&id("P"), &all, true).unwrap();
        let ids: Vec<_> = shown.visible.iter().map(|r| r.task.id.clone()).collect();
        assert_eq!(ids, vec![id("C"), id("S")]);
    }

    #[test]
    fn detail_reflects_reloaded_tasks() {
        let before = vec![task("P", None, 0), task("C", Some("P"), 0), task("S", Some("P"), 1)];
        assert_eq!(detail(&id("P"), &before, false).unwrap().progress, Some(Progress { done: 0, total: 2 }));
        let mut after = before.clone();
        after[1].checked = true;
        assert_eq!(detail(&id("P"), &after, false).unwrap().progress, Some(Progress { done: 1, total: 2 }));
    }

    #[test]
    fn detail_none_for_missing() {
        assert!(detail(&id("X"), &[task("A", None, 0)], false).is_none());
    }

    #[test]
    fn detail_has_parent() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0)];
        let d = detail(&id("C"), &all, false).unwrap();
        assert_eq!(d.parent.unwrap().id, id("P"));
        assert!(detail(&id("P"), &all, false).unwrap().parent.is_none());
    }

    #[test]
    fn visible_rows_carry_their_own_progress() {
        let all = vec![task("P", None, 0), task("C", Some("P"), 0), task("G", Some("C"), 0)];
        let d = detail(&id("P"), &all, false).unwrap();
        assert_eq!(d.visible[0].progress, Some(Progress { done: 0, total: 1 }));
    }
}
