use rusqlite::Connection;

/// Idempotent schema creation. Safe to run on every open.
pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);

        CREATE TABLE IF NOT EXISTS project (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            color       TEXT NOT NULL,
            icon        TEXT,
            parent_id   TEXT,
            child_order INTEGER NOT NULL,
            order_key   TEXT,
            view_style  TEXT NOT NULL,
            is_favorite INTEGER NOT NULL,
            is_collapsed INTEGER NOT NULL,
            is_archived INTEGER NOT NULL,
            is_inbox    INTEGER NOT NULL,
            is_deleted  INTEGER NOT NULL,
            added_at    TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task (
            id            TEXT PRIMARY KEY,
            project_id    TEXT NOT NULL,
            section_id    TEXT,
            parent_id     TEXT,
            content       TEXT NOT NULL,
            description   TEXT NOT NULL,
            priority      INTEGER NOT NULL,
            due_json      TEXT NOT NULL,
            deadline      TEXT,
            duration_json TEXT NOT NULL,
            labels_json   TEXT NOT NULL,
            child_order   INTEGER NOT NULL,
            order_key     TEXT,
            day_order     INTEGER NOT NULL,
            is_collapsed  INTEGER NOT NULL,
            checked       INTEGER NOT NULL,
            is_deleted    INTEGER NOT NULL,
            added_at      TEXT NOT NULL,
            completed_at  TEXT,
            updated_at    TEXT NOT NULL,
            streak_enabled    INTEGER NOT NULL,
            streak_current    INTEGER NOT NULL,
            streak_best       INTEGER NOT NULL,
            streak_goal       INTEGER,
            streak_last_period TEXT
        );
        CREATE INDEX IF NOT EXISTS task_by_project ON task (project_id, is_deleted);
        CREATE INDEX IF NOT EXISTS task_by_parent  ON task (parent_id);

        CREATE TABLE IF NOT EXISTS label (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            color       TEXT NOT NULL,
            item_order  INTEGER NOT NULL,
            is_favorite INTEGER NOT NULL,
            is_deleted  INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS filter (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            query       TEXT NOT NULL,
            color       TEXT NOT NULL,
            item_order  INTEGER NOT NULL,
            is_favorite INTEGER NOT NULL,
            is_deleted  INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS section (
            id            TEXT PRIMARY KEY,
            project_id    TEXT NOT NULL,
            name          TEXT NOT NULL,
            description   TEXT,
            section_order INTEGER NOT NULL,
            order_key     TEXT,
            is_collapsed  INTEGER NOT NULL,
            is_archived   INTEGER NOT NULL,
            is_deleted    INTEGER NOT NULL,
            added_at      TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS reminder (
            id            TEXT PRIMARY KEY,
            task_id       TEXT NOT NULL,
            kind          TEXT NOT NULL,
            due_json      TEXT NOT NULL,
            minute_offset INTEGER,
            is_deleted    INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS command_log (
            uuid       TEXT PRIMARY KEY,
            type       TEXT NOT NULL,
            args_json  TEXT NOT NULL,
            created_at TEXT NOT NULL,
            applied    INTEGER NOT NULL DEFAULT 1,
            synced     INTEGER NOT NULL DEFAULT 0
        );

        -- Device-local preferences (e.g. notifications); never synced.
        CREATE TABLE IF NOT EXISTS local_setting (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS streak_event (
            id         TEXT PRIMARY KEY,
            task_id    TEXT NOT NULL,
            period_key TEXT NOT NULL,
            kind       TEXT NOT NULL,
            at         TEXT NOT NULL
        );
        "#,
    )
}
