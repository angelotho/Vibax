//! Banco SQLite local com a fila e o histórico de downloads.
//!
//! Cada tarefa é guardada como JSON (flexível para evoluir o modelo) com
//! colunas indexadas para as consultas da fila e do histórico.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::downloads::job::Job;
use crate::error::AppResult;

pub struct Db {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS jobs (
    id          TEXT PRIMARY KEY,
    data        TEXT NOT NULL,
    status      TEXT NOT NULL,
    title       TEXT,
    created_at  INTEGER NOT NULL,
    finished_at INTEGER,
    in_queue    INTEGER NOT NULL DEFAULT 1,
    in_history  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_jobs_queue   ON jobs(in_queue, created_at);
CREATE INDEX IF NOT EXISTS idx_jobs_history ON jobs(in_history, finished_at);
PRAGMA user_version = 1;
"#;

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn upsert(&self, job: &Job) -> AppResult<()> {
        let data = serde_json::to_string(job)?;
        self.conn().execute(
            "INSERT INTO jobs (id, data, status, title, created_at, finished_at, in_queue, in_history)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET data=excluded.data, status=excluded.status, title=excluded.title,
               finished_at=excluded.finished_at, in_queue=excluded.in_queue, in_history=excluded.in_history",
            params![
                job.id,
                data,
                job.status.as_str(),
                job.title,
                job.created_at,
                job.finished_at,
                job.in_queue as i32,
                job.in_history as i32
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        self.conn().execute("DELETE FROM jobs WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> AppResult<Option<Job>> {
        let data: Option<String> = self
            .conn()
            .query_row("SELECT data FROM jobs WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?;
        Ok(data.and_then(|d| serde_json::from_str(&d).ok()))
    }

    fn query(&self, sql: &str, p: &[&dyn rusqlite::ToSql]) -> AppResult<Vec<Job>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(p, |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            // Um registro corrompido não impede a leitura dos demais.
            if let Ok(job) = serde_json::from_str::<Job>(&row?) {
                out.push(job);
            }
        }
        Ok(out)
    }

    pub fn queue(&self) -> AppResult<Vec<Job>> {
        self.query("SELECT data FROM jobs WHERE in_queue = 1 ORDER BY created_at ASC", &[])
    }

    pub fn history(&self, search: Option<&str>, limit: u32) -> AppResult<Vec<Job>> {
        let limit = limit.clamp(1, 5000) as i64;
        match search.map(str::trim).filter(|s| !s.is_empty()) {
            Some(q) => {
                let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
                let like = format!("%{escaped}%");
                self.query(
                    "SELECT data FROM jobs WHERE in_history = 1 AND (title LIKE ?1 ESCAPE '\\' OR data LIKE ?1 ESCAPE '\\')
                     ORDER BY finished_at DESC LIMIT ?2",
                    &[&like, &limit],
                )
            }
            None => self.query(
                "SELECT data FROM jobs WHERE in_history = 1 ORDER BY finished_at DESC LIMIT ?1",
                &[&limit],
            ),
        }
    }

    /// Remove do histórico; apaga o registro se ele também não estiver na fila.
    pub fn remove_from_history(&self, id: &str) -> AppResult<Option<Job>> {
        let Some(mut job) = self.get(id)? else { return Ok(None) };
        job.in_history = false;
        if job.in_queue {
            self.upsert(&job)?;
        } else {
            self.delete(id)?;
        }
        Ok(Some(job))
    }

    pub fn clear_history(&self) -> AppResult<Vec<String>> {
        let conn = self.conn();
        let ids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM jobs WHERE in_history = 1")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.filter_map(Result::ok).collect()
        };
        conn.execute("DELETE FROM jobs WHERE in_history = 1 AND in_queue = 0", [])?;
        conn.execute(
            "UPDATE jobs SET in_history = 0, data = json_set(data, '$.inHistory', json('false')) WHERE in_history = 1",
            [],
        )?;
        Ok(ids)
    }
}
