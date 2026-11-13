use chrono::NaiveDate;
use keel_core::commands::Action;
use keel_core::ids::Id;
use keel_core::model::{Due, Priority};
use keel_core::search::SearchScope;

mod common;
use common::{d, Fx, NewTask};

fn today() -> NaiveDate {
    d(2026, 10, 19)
}

impl Fx {
    fn add(&mut self, name: &str, description: &str, due: Option<&str>, priority: Priority) -> Id {
        let due = due.map(|d| Due {
            date: d.into(),
            timezone: None,
            string: d.into(),
            lang: "en".into(),
            is_recurring: false,
            recurrence: None,
            until: None,
            moved_from: None,
        });
        self.add_task(
            NewTask::new(name)
                .description(description)
                .priority(priority)
                .due(due),
        )
    }

    /// A task in `project` (or the Inbox) carrying `labels`.
    fn add_in(
        &mut self,
        name: &str,
        project: Option<&Id>,
        labels: &[&str],
        due: Option<&str>,
        priority: Priority,
    ) -> Id {
        let id = self.add(name, "", due, priority);
        if let Some(p) = project {
            self.run(Action::TaskMove {
                id: id.clone(),
                project_id: p.clone(),
                section_id: None,
            });
        }
        if !labels.is_empty() {
            self.run(Action::TaskUpdate {
                id: id.clone(),
                content: None,
                description: None,
                priority: None,
                project_id: None,
                labels: Some(labels.iter().map(|l| l.to_string()).collect()),
                due: None,
                is_collapsed: None,
                deadline: None,
            });
        }
        id
    }
}

fn names(tasks: &[keel_core::model::Task]) -> Vec<&str> {
    tasks.iter().map(|t| t.content.as_str()).collect()
}

#[test]
fn title_matches_first_then_due_then_priority() {
    let mut f = Fx::on(today());
    f.add(
        "Groceries",
        "milk and eggs",
        Some("2026-10-19"),
        Priority::None,
    );
    f.add("Milk run later", "", Some("2026-10-25"), Priority::None);
    f.add("Buy milk", "", Some("2026-10-20"), Priority::None);
    f.add("Milk the goat", "", None, Priority::P1);
    f.add("Bread", "", Some("2026-10-19"), Priority::None);
    let r = f.svc.search_at("milk", None, today()).unwrap();
    assert_eq!(
        names(&r.open),
        vec!["Buy milk", "Milk run later", "Milk the goat", "Groceries"]
    );
    assert!(r.completed.is_empty());
}

#[test]
fn completed_matches_are_separate_and_empty_query_finds_nothing() {
    let mut f = Fx::on(today());
    let done = f.add("Call the bank", "", None, Priority::None);
    f.add("Bank statement", "", None, Priority::None);
    f.complete(&done);
    let r = f.svc.search_at("bank", None, today()).unwrap();
    assert_eq!(names(&r.open), vec!["Bank statement"]);
    assert_eq!(names(&r.completed), vec!["Call the bank"]);
    assert_eq!(
        f.svc.search_at("   ", None, today()).unwrap(),
        Default::default()
    );
}

#[test]
fn filter_terms_narrow_the_search() {
    let mut f = Fx::on(today());
    f.run(Action::LabelAdd {
        name: "errands".into(),
    });
    f.add_in(
        "Buy milk",
        None,
        &["errands"],
        Some("2026-10-19"),
        Priority::P1,
    );
    f.add_in(
        "Buy bread",
        None,
        &["errands"],
        Some("2026-10-25"),
        Priority::None,
    );
    f.add_in("Buy stamps", None, &[], Some("2026-10-19"), Priority::None);
    let open = |f: &Fx, q: &str| {
        names(&f.svc.search_at(q, None, today()).unwrap().open)
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(open(&f, "buy & @errands & today"), vec!["Buy milk"]);
    assert_eq!(
        open(&f, "buy p1"),
        vec!["Buy milk"],
        "words are ANDed, as in filters"
    );
    assert_eq!(open(&f, "buy & !@errands"), vec!["Buy stamps"]);
    assert_eq!(open(&f, "bread | stamps").len(), 2);
}

#[test]
fn a_query_that_does_not_parse_searches_as_text() {
    let mut f = Fx::on(today());
    f.add("Plan p1 & beyond", "", None, Priority::None);
    f.add("Other", "", None, Priority::None);
    // "p1 &" is not a filter (the & has nothing after it), so it is read as words.
    let r = f.svc.search_at("p1 &", None, today()).unwrap();
    assert_eq!(names(&r.open), vec!["Plan p1 & beyond"]);
}

#[test]
fn comma_groups_are_one_list() {
    let mut f = Fx::on(today());
    f.add("Alpha", "", Some("2026-10-19"), Priority::None);
    f.add("Beta", "", None, Priority::P1);
    f.add("Gamma", "", None, Priority::None);
    let r = f.svc.search_at("today, p1", None, today()).unwrap();
    assert_eq!(names(&r.open), vec!["Alpha", "Beta"]);
}

#[test]
fn filters_apply_to_completed_tasks_too() {
    let mut f = Fx::on(today());
    let done = f.add("Call the bank", "", None, Priority::P1);
    let other = f.add("Call the vet", "", None, Priority::None);
    f.complete(&done);
    f.complete(&other);
    let r = f.svc.search_at("call p1", None, today()).unwrap();
    assert_eq!(names(&r.completed), vec!["Call the bank"]);
}

#[test]
fn a_project_scope_limits_every_group() {
    let mut f = Fx::on(today());
    let home = f
        .run(Action::ProjectAdd {
            name: "Home".into(),
        })
        .id_mapping["t"]
        .clone();
    f.run(Action::ProjectAdd {
        name: "Work".into(),
    });
    f.add_in("Fix sink", Some(&home), &[], None, Priority::None);
    f.add_in("Fix report", None, &[], None, Priority::None);
    let scope = SearchScope::Project { id: home.clone() };
    let r = f.svc.search_at("fix", Some(&scope), today()).unwrap();
    assert_eq!(names(&r.open), vec!["Fix sink"]);
    let both = f
        .svc
        .search_at("sink, report", Some(&scope), today())
        .unwrap();
    assert_eq!(
        names(&both.open),
        vec!["Fix sink"],
        "the scope holds for each comma group"
    );
}

#[test]
fn a_label_scope_limits_the_search() {
    let mut f = Fx::on(today());
    f.run(Action::LabelAdd {
        name: "Errands".into(),
    });
    f.add_in("Buy milk", None, &["Errands"], None, Priority::None);
    f.add_in("Buy desk", None, &[], None, Priority::None);
    let scope = SearchScope::Label {
        name: "errands".into(),
    };
    let r = f.svc.search_at("buy", Some(&scope), today()).unwrap();
    assert_eq!(
        names(&r.open),
        vec!["Buy milk"],
        "label names match without regard to case"
    );
}
