use crate::model::{CodeSnippet, Solution};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ProjectStat {
    pub path: String,
    pub count: usize,
}

pub struct ToolStat {
    pub name: String,
    pub count: usize,
}

pub struct AppStats {
    pub total_solutions: usize,
    pub pinned_solutions: usize,
    pub total_commands: usize,
    pub total_snippets: usize,
    pub top_projects: Vec<ProjectStat>,
    pub top_tools: Vec<ToolStat>,
    pub top_languages: Vec<ToolStat>,
}

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

    pub fn is_empty(&self) -> Result<bool> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM solutions")?;
        let count: i64 = stmt.query_row([], |r| r.get(0))?;
        Ok(count == 0)
    }

    pub fn insert_solutions_batch(&self, solutions: &[Solution]) -> Result<usize> {
        if solutions.is_empty() {
            return Ok(0);
        }

        self.conn.execute("BEGIN TRANSACTION;", [])?;

        let mut stmt = self.conn.prepare(
            "INSERT OR IGNORE INTO solutions (prompt_summary, commands, code_snippets, project_path, git_repo, is_pinned, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        let mut inserted = 0;
        for solution in solutions {
            let commands_json = serde_json::to_string(&solution.commands)?;
            let snippets_json = serde_json::to_string(&solution.code_snippets)?;

            let res = stmt.execute(params![
                solution.prompt_summary,
                commands_json,
                snippets_json,
                solution.project_path,
                solution.git_repo,
                solution.is_pinned as i32,
                solution.timestamp,
            ]);

            if let Ok(changes) = res {
                if changes > 0 {
                    inserted += 1;
                }
            }
        }

        self.conn.execute("COMMIT;", [])?;
        Ok(inserted)
    }

    pub fn search_paged(
        &self,
        query: Option<&str>,
        project_path: Option<&str>,
        pinned_only: bool,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Solution>> {
        let mut sql = String::from(
            "SELECT id, prompt_summary, commands, code_snippets, project_path, git_repo, is_pinned, timestamp FROM solutions WHERE 1=1"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Only return items with executable commands or code snippets
        sql.push_str(" AND (commands != '[]' OR code_snippets != '[]')");

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

        sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

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

    pub fn get_total_count(&self, query: Option<&str>, project_path: Option<&str>, pinned_only: bool) -> Result<usize> {
        let mut sql = String::from("SELECT COUNT(*) FROM solutions WHERE 1=1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Only count items with executable commands or code snippets
        sql.push_str(" AND (commands != '[]' OR code_snippets != '[]')");

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

        let mut stmt = self.conn.prepare(&sql)?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let count: i64 = stmt.query_row(rusqlite_params.as_slice(), |r| r.get(0))?;
        Ok(count as usize)
    }

    pub fn get_stats(&self) -> Result<AppStats> {
        let total_solutions = self.get_total_count(None, None, false)?;
        let pinned_solutions = self.get_total_count(None, None, true)?;

        // Top Projects
        let mut proj_stmt = self.conn.prepare(
            "SELECT project_path, COUNT(*) as cnt FROM solutions 
             WHERE (commands != '[]' OR code_snippets != '[]') 
             GROUP BY project_path ORDER BY cnt DESC LIMIT 5"
        )?;
        let proj_rows = proj_stmt.query_map([], |r| {
            let path: String = r.get(0)?;
            let count: i64 = r.get(1)?;
            Ok(ProjectStat { path, count: count as usize })
        })?;
        let mut top_projects = Vec::new();
        for pr in proj_rows {
            top_projects.push(pr?);
        }

        // Aggregate Commands & Languages Frequency
        let mut all_stmt = self.conn.prepare(
            "SELECT commands, code_snippets FROM solutions WHERE (commands != '[]' OR code_snippets != '[]')"
        )?;
        let mut total_commands = 0;
        let mut total_snippets = 0;
        let mut tool_freq: HashMap<String, usize> = HashMap::new();
        let mut lang_freq: HashMap<String, usize> = HashMap::new();

        let rows = all_stmt.query_map([], |r| {
            let cmds_json: String = r.get(0)?;
            let snips_json: String = r.get(1)?;
            Ok((cmds_json, snips_json))
        })?;

        for r in rows {
            let (cmds_json, snips_json) = r?;
            let cmds: Vec<String> = serde_json::from_str(&cmds_json).unwrap_or_default();
            let snips: Vec<CodeSnippet> = serde_json::from_str(&snips_json).unwrap_or_default();

            total_commands += cmds.len();
            total_snippets += snips.len();

            for cmd in cmds {
                let first_word = cmd
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_lowercase();

                let tool_name = match first_word.as_str() {
                    "cargo" | "git" | "docker" | "npm" | "npx" | "kubectl" | "python" | "python3"
                    | "pip" | "go" | "rustc" | "ffmpeg" | "curl" | "wget" | "cpl" | "gh" | "agy" => {
                        first_word
                    }
                    _ => continue,
                };
                *tool_freq.entry(tool_name).or_insert(0) += 1;
            }

            for snip in snips {
                let lang = snip.language.to_lowercase();
                if !lang.is_empty() && lang != "text" {
                    *lang_freq.entry(lang).or_insert(0) += 1;
                }
            }
        }

        let mut top_tools: Vec<ToolStat> = tool_freq
            .into_iter()
            .map(|(name, count)| ToolStat { name, count })
            .collect();
        top_tools.sort_by(|a, b| b.count.cmp(&a.count));
        top_tools.truncate(5);

        let mut top_languages: Vec<ToolStat> = lang_freq
            .into_iter()
            .map(|(name, count)| ToolStat { name, count })
            .collect();
        top_languages.sort_by(|a, b| b.count.cmp(&a.count));
        top_languages.truncate(5);

        Ok(AppStats {
            total_solutions,
            pinned_solutions,
            total_commands,
            total_snippets,
            top_projects,
            top_tools,
            top_languages,
        })
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

    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM solutions", [])?;
        let _ = self.conn.execute("DELETE FROM solutions_fts", []);
        Ok(())
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
