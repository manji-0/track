use rusqlite::params;
use chrono::Utc;
use crate::db::Database;
use crate::models::{Link, Scrap};
use crate::utils::{Result, TrackError};

pub struct LinkService<'a> {
    db: &'a Database,
}

impl<'a> LinkService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn add_link(&self, task_id: i64, url: &str, title: Option<&str>) -> Result<Link> {
        self.validate_url(url)?;
        
        let title = title.unwrap_or(url);
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO links (task_id, url, title, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![task_id, url, title, now],
        )?;

        let link_id = conn.last_insert_rowid();
        self.get_link(link_id)
    }

    pub fn get_link(&self, link_id: i64) -> Result<Link> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, url, title, created_at FROM links WHERE id = ?1"
        )?;

        let link = stmt.query_row(params![link_id], |row| {
            Ok(Link {
                id: row.get(0)?,
                task_id: row.get(1)?,
                url: row.get(2)?,
                title: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        })?;

        Ok(link)
    }

    pub fn list_links(&self, task_id: i64) -> Result<Vec<Link>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, url, title, created_at FROM links WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;

        let links = stmt.query_map(params![task_id], |row| {
            Ok(Link {
                id: row.get(0)?,
                task_id: row.get(1)?,
                url: row.get(2)?,
                title: row.get(3)?,
                created_at: row.get::<_, String>(4)?.parse().unwrap(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(links)
    }

    fn validate_url(&self, url: &str) -> Result<()> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(())
        } else {
            Err(TrackError::InvalidUrl(url.to_string()))
        }
    }
}

pub struct ScrapService<'a> {
    db: &'a Database,
}

impl<'a> ScrapService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn add_scrap(&self, task_id: i64, content: &str) -> Result<Scrap> {
        let now = Utc::now().to_rfc3339();
        let conn = self.db.get_connection();

        conn.execute(
            "INSERT INTO scraps (task_id, content, created_at) VALUES (?1, ?2, ?3)",
            params![task_id, content, now],
        )?;

        let scrap_id = conn.last_insert_rowid();
        self.get_scrap(scrap_id)
    }

    pub fn get_scrap(&self, scrap_id: i64) -> Result<Scrap> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, content, created_at FROM scraps WHERE id = ?1"
        )?;

        let scrap = stmt.query_row(params![scrap_id], |row| {
            Ok(Scrap {
                id: row.get(0)?,
                task_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get::<_, String>(3)?.parse().unwrap(),
            })
        })?;

        Ok(scrap)
    }

    pub fn list_scraps(&self, task_id: i64) -> Result<Vec<Scrap>> {
        let conn = self.db.get_connection();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, content, created_at FROM scraps WHERE task_id = ?1 ORDER BY created_at DESC"
        )?;

        let scraps = stmt.query_map(params![task_id], |row| {
            Ok(Scrap {
                id: row.get(0)?,
                task_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get::<_, String>(3)?.parse().unwrap(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(scraps)
    }
}
