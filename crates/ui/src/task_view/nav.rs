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
}

pub fn provide_nav() -> TaskViewNav {
    use_context_provider(|| TaskViewNav {
        stack: Signal::new(ViewStack::default()),
        show_completed: Signal::new(false),
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
