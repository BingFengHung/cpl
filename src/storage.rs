use crate::model::{CodeSnippet, Solution};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let db_path = Self::get_db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", db_path))?;

        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find user home directory")?;
        Ok(home.join(".cpl").join("recall.db"))
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS solutions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prompt_summary TEXT NOT NULL,
                commands TEXT NOT NULL,
                code_snippets TEXT NOT NULL,
                project_path TEXT NOT NULL,
                git_repo TEXT,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                timestamp INTEGER NOT NULL,
                UNIQUE(prompt_summary, project_path, timestamp)
            );",
            [],
        )?;

        // Try initializing FTS5 table for full-text search
        let _ = self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS solutions_fts USING fts5(
                prompt_summary,
                commands,
                code_snippets,
                content='solutions',
                content_rowid='id'
            );",
            [],
        );

        // Triggers to keep FTS index synchronized
        let _ = self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS solutions_ai AFTER INSERT ON solutions BEGIN
                INSERT INTO solutions_fts(rowid, prompt_summary, commands, code_snippets)
                VALUES (new.id, new.prompt_summary, new.commands, new.code_snippets);
            END;",
            [],
        );

        let _ = self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS solutions_ad AFTER DELETE ON solutions BEGIN
                INSERT INTO solutions_fts(solutions_fts, rowid, prompt_summary, commands, code_snippets)
                VALUES('delete', old.id, old.prompt_summary, old.commands, old.code_snippets);
            END;",
            [],
        );

        let _ = self.conn.execute(
            "CREATE TRIGGER IF NOT EXISTS solutions_au AFTER UPDATE ON solutions BEGIN
                INSERT INTO solutions_fts(solutions_fts, rowid, prompt_summary, commands, code_snippets)
                VALUES('delete', old.id, old.prompt_summary, old.commands, old.code_snippets);
                INSERT INTO solutions_fts(rowid, prompt_summary, commands, code_snippets)
                VALUES (new.id, new.prompt_summary, new.commands, new.code_snippets);
            END;",
            [],
        );

        Ok(())
    }

    pub fn insert_solution(&self, solution: &Solution) -> Result<i64> {
        let commands_json = serde_json::to_string(&solution.commands)?;
        let snippets_json = serde_json::to_string(&solution.code_snippets)?;

        self.conn.execute(
            "INSERT OR IGNORE INTO solutions (prompt_summary, commands, code_snippets, project_path, git_repo, is_pinned, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                solution.prompt_summary,
                commands_json,
                snippets_json,
                solution.project_path,
                solution.git_repo,
                solution.is_pinned as i32,
                solution.timestamp,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn search(
        &self,
        query: Option<&str>,
        project_path: Option<&str>,
        pinned_only: bool,
    ) -> Result<Vec<Solution>> {
        let mut sql = String::from(
            "SELECT id, prompt_summary, commands, code_snippets, project_path, git_repo, is_pinned, timestamp FROM solutions WHERE 1=1"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if pinned_only {
            sql.push_str(" AND is_pinned = 1");
        }

        if let Some(path) = project_path {
            sql.push_str(" AND project_path = ?");
            params_vec.push(Box::new(path.to_string()));
        }

        if let Some(q) = query {
            if !q.trim().is_empty() {
                sql.push_str(" AND (prompt_summary LIKE ? OR commands LIKE ? OR code_snippets LIKE ?)");
                let pattern = format!("%{}%", q.trim());
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern.clone()));
            }
        }

        sql.push_str(" ORDER BY timestamp DESC LIMIT 100");

        let mut stmt = self.conn.prepare(&sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(rusqlite_params.as_slice(), |row| {
            let id: i64 = row.get(0)?;
            let prompt_summary: String = row.get(1)?;
            let commands_json: String = row.get(2)?;
            let snippets_json: String = row.get(3)?;
            let project_path: String = row.get(4)?;
            let git_repo: Option<String> = row.get(5)?;
            let is_pinned_int: i32 = row.get(6)?;
            let timestamp: i64 = row.get(7)?;

            let commands: Vec<String> = serde_json::from_str(&commands_json).unwrap_or_default();
            let code_snippets: Vec<CodeSnippet> = serde_json::from_str(&snippets_json).unwrap_or_default();

            Ok(Solution {
                id,
                prompt_summary,
                commands,
                code_snippets,
                project_path,
                git_repo,
                is_pinned: is_pinned_int != 0,
                timestamp,
            })
        })?;

        let mut solutions = Vec::new();
        for r in rows {
            solutions.push(r?);
        }
        Ok(solutions)
    }

    pub fn toggle_pin(&self, id: i64) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT is_pinned FROM solutions WHERE id = ?1")?;
        let current_pinned: i32 = stmt.query_row(params![id], |row| row.get(0))?;
        let new_pinned = if current_pinned == 0 { 1 } else { 0 };

        self.conn.execute(
            "UPDATE solutions SET is_pinned = ?1 WHERE id = ?2",
            params![new_pinned, id],
        )?;

        Ok(new_pinned == 1)
    }

    pub fn get_latest_id(&self) -> Result<Option<i64>> {
        let mut stmt = self.conn.prepare("SELECT id FROM solutions ORDER BY id DESC LIMIT 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }
}
