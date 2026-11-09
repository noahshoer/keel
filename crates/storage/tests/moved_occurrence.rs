// Moving one occurrence of a repeating task (issue #8c): the repeat stays.
// 2026-10-21 is a Wednesday.
use chrono::NaiveDate;
use keel_core::commands::{apply_at, Action, Command, DueEdit};
use keel_core::ids::Id;
use keel_core::model::{Due, Task};
use keel_core::recurrence::Recurrence;
use keel_core::repository::Repository;
use keel_storage::SqliteRepo;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

struct World {
    repo: SqliteRepo,
    n: u32,
}

impl World {
    fn new() -> Self {
        World {
            repo: SqliteRepo::open_in_memory().unwrap(),
            n: 0,
        }
    }

    fn run(&mut self, action: Action, today: NaiveDate) -> keel_core::commands::ApplyResult {
        self.n += 1;
        let cmd = Command {
            uuid: format!("u{}", self.n),
            temp_id: Some("tmp".to_string()),
            action,
        };
        apply_at(&self.repo, cmd, today).unwrap()
    }

    fn add(&mut self, date: &str, rule: Recurrence, until: Option<&str>) -> Id {
        let due = Due {
            date: date.to_string(),
            timezone: None,
            string: String::new(),
            lang: "en".to_string(),
            is_recurring: true,
            recurrence: Some(rule.clone()),
            until: until.map(str::to_string),
            moved_from: None,
        };
        let action = Action::TaskAdd {
            content: "x".to_string(),
            project_id: Id::new(),
            description: None,
            priority: None,
            due: Some(due),
            labels: None,
            parent_id: None,
            section_id: None,
            recurrence: Some(rule),
            deadline: None,
        };
        self.run(action, d(2026, 10, 19)).id_mapping["tmp"].clone()
    }

    fn get(&self, id: &Id) -> Task {
        self.repo.get_task(id).unwrap().unwrap()
    }

    fn due(&self, id: &Id) -> (String, Option<String>) {
        let due = self.get(id).due.unwrap();
        (due.date, due.moved_from)
    }

    fn set_due(&mut self, id: &Id, date: &str, rule: Recurrence) {
        let edit = DueEdit {
            date: Some(date.to_string()),
            recurrence: Some(rule),
            until: None,
        };
        let action = Action::TaskUpdate {
            id: id.clone(),
            content: None,
            description: None,
            priority: None,
            project_id: None,
            due: Some(edit),
            labels: None,
            is_collapsed: None,
            deadline: None,
        };
        self.run(action, d(2026, 10, 19));
    }

    fn complete(&mut self, id: &Id, today: NaiveDate) {
        self.run(Action::TaskComplete { id: id.clone() }, today);
    }
}

fn wed() -> Recurrence {
    Recurrence::Weekly(vec![2])
}

fn due(date: &str, from: Option<&str>) -> (String, Option<String>) {
    (date.to_string(), from.map(str::to_string))
}

#[test]
fn moving_a_date_keeps_the_repeat_and_remembers_the_occurrence() {
    let mut w = World::new();
    let id = w.add("2026-10-21", wed(), None);
    w.set_due(&id, "2026-10-22", wed());
    assert_eq!(w.due(&id), due("2026-10-22", Some("2026-10-21")));
    assert_eq!(w.get(&id).due.unwrap().recurrence, Some(wed()));
    // Moved again, it still stands in for Wednesday's.
    w.set_due(&id, "2026-10-23", wed());
    assert_eq!(w.due(&id), due("2026-10-23", Some("2026-10-21")));
    // Back on a real occurrence, it's no longer a move.
    w.set_due(&id, "2026-10-21", wed());
    assert_eq!(w.due(&id), due("2026-10-21", None));
    w.set_due(&id, "2026-10-28", wed());
    assert_eq!(w.due(&id), due("2026-10-28", None));
}

#[test]
fn completing_a_moved_occurrence_goes_on_to_the_next_one() {
    let mut w = World::new();
    let later = w.add("2026-10-21", wed(), None);
    w.set_due(&later, "2026-10-22", wed());
    w.complete(&later, d(2026, 10, 22));
    assert_eq!(w.due(&later), due("2026-10-28", None));

    // Moved earlier, it doesn't come back to the Wednesday it replaced.
    let earlier = w.add("2026-10-21", wed(), None);
    w.set_due(&earlier, "2026-10-20", wed());
    w.complete(&earlier, d(2026, 10, 20));
    assert_eq!(w.due(&earlier), due("2026-10-28", None));
}

#[test]
fn undoing_the_completion_puts_the_move_back() {
    let mut w = World::new();
    let id = w.add("2026-10-21", wed(), None);
    w.set_due(&id, "2026-10-22", wed());
    w.complete(&id, d(2026, 10, 22));
    w.run(Action::TaskUncomplete { id: id.clone() }, d(2026, 10, 22));
    assert_eq!(w.due(&id), due("2026-10-22", Some("2026-10-21")));
}

#[test]
fn a_new_rule_is_not_a_move() {
    let mut w = World::new();
    let id = w.add("2026-10-21", wed(), None);
    w.set_due(&id, "2026-10-22", wed());
    w.set_due(&id, "2026-10-22", Recurrence::Weekly(vec![3]));
    assert_eq!(w.due(&id), due("2026-10-22", None));
}

#[test]
fn a_moved_last_occurrence_still_ends_the_repeat() {
    let mut w = World::new();
    let id = w.add("2026-10-21", Recurrence::Daily, Some("2026-10-21"));
    w.set_due(&id, "2026-10-23", Recurrence::Daily);
    w.complete(&id, d(2026, 10, 23));
    assert!(w.get(&id).checked);
}

#[test]
fn moving_resets_the_streak_and_keeps_the_best() {
    let mut w = World::new();
    let id = w.add("2026-10-19", Recurrence::Daily, None);
    w.run(
        Action::TaskSetStreak {
            id: id.clone(),
            enabled: true,
            goal: None,
        },
        d(2026, 10, 19),
    );
    w.complete(&id, d(2026, 10, 19));
    w.complete(&id, d(2026, 10, 20));
    assert_eq!(w.get(&id).streak_current, 2);
    w.set_due(&id, "2026-10-23", Recurrence::Daily);
    let t = w.get(&id);
    assert_eq!(
        (t.streak_current, t.streak_best, t.streak_last_period),
        (0, 2, None)
    );
}
