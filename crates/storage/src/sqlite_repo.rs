use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use keel_core::ids::Id;
use keel_core::model::{Due, Duration, Filter, Label, Priority, Project, Reminder, ReminderType, Section, Task, ViewStyle};
use keel_core::repository::{RepoError, Repository};

use crate::migrations;

/// SQLite-backed `Repository` (local store for v1).
pub struct SqliteRepo {
    conn: Mutex<Connection>,
}

fn backend<E: std::fmt::Display>(e: E) -> RepoError {
    RepoError::Backend(e.to_string())
}

fn ts(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>, RepoError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(backend)
}

fn opt_id(s: Option<String>) -> Option<Id> {
    s.map(Id::from)
}

impl SqliteRepo {
    pub fn open(path: &str) -> Result<Self, RepoError> {
        let conn = Connection::open(path).map_err(backend)?;
        migrations::run(&conn).map_err(backend)?;
        Ok(SqliteRepo {
            conn: Mutex::new(conn),
        })
    }

    fn query_sections(&self, filter: &str, arg: &str) -> Result<Vec<Section>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let sql = format!(
            "SELECT id, project_id, name, description, section_order, order_key, is_collapsed, \
             is_archived, is_deleted, added_at, updated_at FROM section {filter} \
             ORDER BY section_order ASC, added_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(backend)?;
        let rows = stmt
            .query_map(params![arg], |row| {
                let added: String = row.get(9)?;
                let updated: String = row.get(10)?;
                Ok((
                    Section {
                        id: Id::from(row.get::<_, String>(0)?),
                        project_id: Id::from(row.get::<_, String>(1)?),
                        name: row.get(2)?,
                        description: row.get(3)?,
                        section_order: row.get(4)?,
                        order_key: row.get(5)?,
                        is_collapsed: row.get::<_, i64>(6)? != 0,
                        is_archived: row.get::<_, i64>(7)? != 0,
                        is_deleted: row.get::<_, i64>(8)? != 0,
                        added_at: Utc::now(),
                        updated_at: Utc::now(),
                    },
                    added,
                    updated,
                ))
            })
            .map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            let (mut s, added, updated) = r.map_err(backend)?;
            s.added_at = parse_ts(&added)?;
            s.updated_at = parse_ts(&updated)?;
            out.push(s);
        }
        Ok(out)
    }

    /// A device-local setting (`local_setting`; never synced).
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let mut stmt = conn.prepare("SELECT value FROM local_setting WHERE key = ?1").map_err(backend)?;
        let mut rows = stmt.query(params![key]).map_err(backend)?;
        match rows.next().map_err(backend)? {
            Some(row) => Ok(Some(row.get(0).map_err(backend)?)),
            None => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        conn.execute(
            "INSERT INTO local_setting (key, value) VALUES (?1, ?2)              ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(backend)?;
        Ok(())
    }

    /// Open an in-memory database (used by tests/tools).
    pub fn open_in_memory() -> Result<Self, RepoError> {
        let conn = Connection::open_in_memory().map_err(backend)?;
        migrations::run(&conn).map_err(backend)?;
        Ok(SqliteRepo {
            conn: Mutex::new(conn),
        })
    }
}

fn view_style_to_str(v: ViewStyle) -> &'static str {
    match v {
        ViewStyle::List => "list",
        ViewStyle::Board => "board",
        ViewStyle::Calendar => "calendar",
    }
}

fn view_style_from_str(s: &str) -> ViewStyle {
    match s {
        "board" => ViewStyle::Board,
        "calendar" => ViewStyle::Calendar,
        _ => ViewStyle::List,
    }
}

fn row_to_task(row: &Row<'_>) -> rusqlite::Result<Task> {
    let due_json: String = row.get("due_json")?;
    let duration_json: String = row.get("duration_json")?;
    let labels_json: String = row.get("labels_json")?;
    let priority_api: i64 = row.get("priority")?;
    let added_at: String = row.get("added_at")?;
    let updated_at: String = row.get("updated_at")?;
    let completed_at: Option<String> = row.get("completed_at")?;

    let map_json = |e: serde_json::Error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    };
    let map_ts = |s: &str| {
        parse_ts(s).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
            )
        })
    };

    Ok(Task {
        id: Id::from(row.get::<_, String>("id")?),
        project_id: Id::from(row.get::<_, String>("project_id")?),
        section_id: opt_id(row.get("section_id")?),
        parent_id: opt_id(row.get("parent_id")?),
        content: row.get("content")?,
        description: row.get("description")?,
        priority: Priority::from_api(priority_api as u8),
        due: serde_json::from_str::<Option<Due>>(&due_json).map_err(map_json)?,
        deadline: row.get("deadline")?,
        duration: serde_json::from_str::<Option<Duration>>(&duration_json).map_err(map_json)?,
        labels: serde_json::from_str::<Vec<String>>(&labels_json).map_err(map_json)?,
        child_order: row.get("child_order")?,
        order_key: row.get("order_key")?,
        day_order: row.get("day_order")?,
        is_collapsed: row.get::<_, i64>("is_collapsed")? != 0,
        checked: row.get::<_, i64>("checked")? != 0,
        is_deleted: row.get::<_, i64>("is_deleted")? != 0,
        added_at: map_ts(&added_at)?,
        completed_at: match completed_at {
            Some(s) => Some(map_ts(&s)?),
            None => None,
        },
        updated_at: map_ts(&updated_at)?,
        streak_enabled: row.get::<_, i64>("streak_enabled")? != 0,
        streak_current: row.get("streak_current")?,
        streak_best: row.get("streak_best")?,
        streak_goal: row.get("streak_goal")?,
        streak_last_period: row.get("streak_last_period")?,
    })
}

fn row_to_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    let added_at: String = row.get("added_at")?;
    let updated_at: String = row.get("updated_at")?;
    let map_ts = |s: &str| {
        parse_ts(s).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
            )
        })
    };
    Ok(Project {
        id: Id::from(row.get::<_, String>("id")?),
        name: row.get("name")?,
        color: row.get("color")?,
        icon: row.get("icon")?,
        parent_id: opt_id(row.get("parent_id")?),
        child_order: row.get("child_order")?,
        order_key: row.get("order_key")?,
        view_style: view_style_from_str(&row.get::<_, String>("view_style")?),
        is_favorite: row.get::<_, i64>("is_favorite")? != 0,
        is_collapsed: row.get::<_, i64>("is_collapsed")? != 0,
        is_archived: row.get::<_, i64>("is_archived")? != 0,
        is_inbox: row.get::<_, i64>("is_inbox")? != 0,
        is_deleted: row.get::<_, i64>("is_deleted")? != 0,
        added_at: map_ts(&added_at)?,
        updated_at: map_ts(&updated_at)?,
    })
}

const TASK_COLS: &str = "id, project_id, section_id, parent_id, content, description, priority, \
    due_json, deadline, duration_json, labels_json, child_order, order_key, day_order, \
    is_collapsed, checked, is_deleted, added_at, completed_at, updated_at, \
    streak_enabled, streak_current, streak_best, streak_goal, streak_last_period";

const PROJECT_COLS: &str = "id, name, color, icon, parent_id, child_order, order_key, view_style, \
    is_favorite, is_collapsed, is_archived, is_inbox, is_deleted, added_at, updated_at";

impl Repository for SqliteRepo {
    fn upsert_task(&self, t: &Task) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let due_json = serde_json::to_string(&t.due).map_err(backend)?;
        let duration_json = serde_json::to_string(&t.duration).map_err(backend)?;
        let labels_json = serde_json::to_string(&t.labels).map_err(backend)?;
        conn.execute(
            "INSERT INTO task (id, project_id, section_id, parent_id, content, description, priority, \
                due_json, deadline, duration_json, labels_json, child_order, order_key, day_order, \
                is_collapsed, checked, is_deleted, added_at, completed_at, updated_at, \
                streak_enabled, streak_current, streak_best, streak_goal, streak_last_period) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25) \
             ON CONFLICT(id) DO UPDATE SET \
                project_id=excluded.project_id, section_id=excluded.section_id, parent_id=excluded.parent_id, \
                content=excluded.content, description=excluded.description, priority=excluded.priority, \
                due_json=excluded.due_json, deadline=excluded.deadline, duration_json=excluded.duration_json, \
                labels_json=excluded.labels_json, child_order=excluded.child_order, order_key=excluded.order_key, \
                day_order=excluded.day_order, is_collapsed=excluded.is_collapsed, checked=excluded.checked, \
                is_deleted=excluded.is_deleted, added_at=excluded.added_at, completed_at=excluded.completed_at, \
                updated_at=excluded.updated_at, streak_enabled=excluded.streak_enabled, \
                streak_current=excluded.streak_current, streak_best=excluded.streak_best, \
                streak_goal=excluded.streak_goal, streak_last_period=excluded.streak_last_period",
            params![
                t.id.as_str(),
                t.project_id.as_str(),
                t.section_id.as_ref().map(|i| i.as_str()),
                t.parent_id.as_ref().map(|i| i.as_str()),
                t.content,
                t.description,
                t.priority.to_api() as i64,
                due_json,
                t.deadline,
                duration_json,
                labels_json,
                t.child_order,
                t.order_key,
                t.day_order,
                t.is_collapsed as i64,
                t.checked as i64,
                t.is_deleted as i64,
                ts(&t.added_at),
                t.completed_at.as_ref().map(ts),
                ts(&t.updated_at),
                t.streak_enabled as i64,
                t.streak_current,
                t.streak_best,
                t.streak_goal,
                t.streak_last_period,
            ],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn get_task(&self, id: &Id) -> Result<Option<Task>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let sql = format!("SELECT {TASK_COLS} FROM task WHERE id = ?1");
        let mut stmt = conn.prepare(&sql).map_err(backend)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(backend)?;
        match rows.next().map_err(backend)? {
            Some(row) => Ok(Some(row_to_task(row).map_err(backend)?)),
            None => Ok(None),
        }
    }

    fn list_tasks(&self, project: &Id) -> Result<Vec<Task>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let sql = format!(
            "SELECT {TASK_COLS} FROM task WHERE project_id = ?1 AND is_deleted = 0 \
             ORDER BY child_order ASC, added_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(backend)?;
        let rows = stmt
            .query_map(params![project.as_str()], |row| row_to_task(row))
            .map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(backend)?);
        }
        Ok(out)
    }

    fn list_all_tasks(&self) -> Result<Vec<Task>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let sql = format!(
            "SELECT {TASK_COLS} FROM task WHERE is_deleted = 0 ORDER BY child_order ASC, added_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(backend)?;
        let rows = stmt.query_map([], |row| row_to_task(row)).map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(backend)?);
        }
        Ok(out)
    }

    fn soft_delete_task(&self, id: &Id) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let now = ts(&Utc::now());
        // Soft-delete the task and its entire sub-tree (children, grandchildren, …).
        conn.execute(
            "UPDATE task SET is_deleted = 1, updated_at = ?2 WHERE id IN (\
                 WITH RECURSIVE sub(id) AS (\
                     SELECT ?1 \
                     UNION ALL \
                     SELECT t.id FROM task t JOIN sub ON t.parent_id = sub.id\
                 ) SELECT id FROM sub\
             )",
            params![id.as_str(), now],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn upsert_project(&self, p: &Project) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        conn.execute(
            "INSERT INTO project (id, name, color, icon, parent_id, child_order, order_key, view_style, \
                is_favorite, is_collapsed, is_archived, is_inbox, is_deleted, added_at, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15) \
             ON CONFLICT(id) DO UPDATE SET \
                name=excluded.name, color=excluded.color, icon=excluded.icon, parent_id=excluded.parent_id, \
                child_order=excluded.child_order, order_key=excluded.order_key, view_style=excluded.view_style, \
                is_favorite=excluded.is_favorite, is_collapsed=excluded.is_collapsed, \
                is_archived=excluded.is_archived, is_inbox=excluded.is_inbox, is_deleted=excluded.is_deleted, \
                added_at=excluded.added_at, updated_at=excluded.updated_at",
            params![
                p.id.as_str(),
                p.name,
                p.color,
                p.icon,
                p.parent_id.as_ref().map(|i| i.as_str()),
                p.child_order,
                p.order_key,
                view_style_to_str(p.view_style),
                p.is_favorite as i64,
                p.is_collapsed as i64,
                p.is_archived as i64,
                p.is_inbox as i64,
                p.is_deleted as i64,
                ts(&p.added_at),
                ts(&p.updated_at),
            ],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn get_project(&self, id: &Id) -> Result<Option<Project>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let sql = format!("SELECT {PROJECT_COLS} FROM project WHERE id = ?1");
        let mut stmt = conn.prepare(&sql).map_err(backend)?;
        let mut rows = stmt.query(params![id.as_str()]).map_err(backend)?;
        match rows.next().map_err(backend)? {
            Some(row) => Ok(Some(row_to_project(row).map_err(backend)?)),
            None => Ok(None),
        }
    }

    fn list_projects(&self) -> Result<Vec<Project>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let sql = format!(
            "SELECT {PROJECT_COLS} FROM project WHERE is_deleted = 0 ORDER BY child_order ASC, added_at ASC"
        );
        let mut stmt = conn.prepare(&sql).map_err(backend)?;
        let rows = stmt
            .query_map([], |row| row_to_project(row))
            .map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(backend)?);
        }
        Ok(out)
    }

    fn upsert_section(&self, s: &Section) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        conn.execute(
            "INSERT INTO section (id, project_id, name, description, section_order, order_key, \
                is_collapsed, is_archived, is_deleted, added_at, updated_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) \
             ON CONFLICT(id) DO UPDATE SET project_id=excluded.project_id, name=excluded.name, \
                description=excluded.description, section_order=excluded.section_order, \
                order_key=excluded.order_key, is_collapsed=excluded.is_collapsed, \
                is_archived=excluded.is_archived, is_deleted=excluded.is_deleted, \
                added_at=excluded.added_at, updated_at=excluded.updated_at",
            params![
                s.id.as_str(),
                s.project_id.as_str(),
                s.name,
                s.description,
                s.section_order,
                s.order_key,
                s.is_collapsed as i64,
                s.is_archived as i64,
                s.is_deleted as i64,
                ts(&s.added_at),
                ts(&s.updated_at),
            ],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn get_section(&self, id: &Id) -> Result<Option<Section>, RepoError> {
        Ok(self.query_sections("WHERE id = ?1", id.as_str())?.into_iter().next())
    }

    fn list_sections(&self, project: &Id) -> Result<Vec<Section>, RepoError> {
        self.query_sections("WHERE project_id = ?1 AND is_deleted = 0", project.as_str())
    }

    fn upsert_filter(&self, f: &Filter) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        conn.execute(
            "INSERT INTO filter (id, name, query, color, item_order, is_favorite, is_deleted) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, query=excluded.query, \
                color=excluded.color, item_order=excluded.item_order, \
                is_favorite=excluded.is_favorite, is_deleted=excluded.is_deleted",
            params![
                f.id.as_str(),
                f.name,
                f.query,
                f.color,
                f.item_order,
                f.is_favorite as i64,
                f.is_deleted as i64,
            ],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn list_filters(&self) -> Result<Vec<Filter>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, query, color, item_order, is_favorite, is_deleted FROM filter \
                 WHERE is_deleted = 0 ORDER BY item_order ASC, rowid ASC",
            )
            .map_err(backend)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Filter {
                    id: Id::from(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    query: row.get(2)?,
                    color: row.get(3)?,
                    item_order: row.get(4)?,
                    is_favorite: row.get::<_, i64>(5)? != 0,
                    is_deleted: row.get::<_, i64>(6)? != 0,
                })
            })
            .map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(backend)?);
        }
        Ok(out)
    }

    fn upsert_label(&self, l: &Label) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        conn.execute(
            "INSERT INTO label (id, name, color, item_order, is_favorite, is_deleted) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, color=excluded.color, \
                item_order=excluded.item_order, is_favorite=excluded.is_favorite, \
                is_deleted=excluded.is_deleted",
            params![
                l.id.as_str(),
                l.name,
                l.color,
                l.item_order,
                l.is_favorite as i64,
                l.is_deleted as i64,
            ],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn upsert_reminder(&self, r: &Reminder) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let due_json = serde_json::to_string(&r.due).map_err(backend)?;
        let kind = match r.kind {
            ReminderType::Relative => "relative",
            ReminderType::Absolute => "absolute",
        };
        conn.execute(
            "INSERT INTO reminder (id, task_id, kind, due_json, minute_offset, is_deleted) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(id) DO UPDATE SET task_id=excluded.task_id, kind=excluded.kind, \
                due_json=excluded.due_json, minute_offset=excluded.minute_offset, \
                is_deleted=excluded.is_deleted",
            params![r.id.as_str(), r.task_id.as_str(), kind, due_json, r.minute_offset, r.is_deleted as i64],
        )
        .map_err(backend)?;
        Ok(())
    }

    fn list_reminders(&self) -> Result<Vec<Reminder>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, kind, due_json, minute_offset, is_deleted FROM reminder \
                 WHERE is_deleted = 0 ORDER BY rowid ASC",
            )
            .map_err(backend)?;
        let rows = stmt
            .query_map([], |row| {
                let due_json: String = row.get(3)?;
                let kind: String = row.get(2)?;
                Ok(Reminder {
                    id: Id::from(row.get::<_, String>(0)?),
                    task_id: Id::from(row.get::<_, String>(1)?),
                    kind: if kind == "absolute" { ReminderType::Absolute } else { ReminderType::Relative },
                    due: serde_json::from_str(&due_json).unwrap_or(None),
                    minute_offset: row.get(4)?,
                    is_deleted: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(backend)?);
        }
        Ok(out)
    }

    fn list_labels(&self) -> Result<Vec<Label>, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, color, item_order, is_favorite, is_deleted FROM label \
                 WHERE is_deleted = 0 ORDER BY name COLLATE NOCASE ASC",
            )
            .map_err(backend)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Label {
                    id: Id::from(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    color: row.get(2)?,
                    item_order: row.get(3)?,
                    is_favorite: row.get::<_, i64>(4)? != 0,
                    is_deleted: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(backend)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(backend)?);
        }
        Ok(out)
    }

    fn has_command(&self, uuid: &str) -> Result<bool, RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        let mut stmt = conn
            .prepare("SELECT 1 FROM command_log WHERE uuid = ?1")
            .map_err(backend)?;
        let exists = stmt.exists(params![uuid]).map_err(backend)?;
        Ok(exists)
    }

    fn record_command(&self, uuid: &str, ctype: &str, args_json: &str) -> Result<(), RepoError> {
        let conn = self.conn.lock().map_err(backend)?;
        conn.execute(
            "INSERT OR IGNORE INTO command_log (uuid, type, args_json, created_at, applied, synced) \
             VALUES (?1, ?2, ?3, ?4, 1, 0)",
            params![uuid, ctype, args_json, ts(&Utc::now())],
        )
        .map_err(backend)?;
        Ok(())
    }
}
