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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::services::TaskService;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    fn create_test_task(db: &Database) -> i64 {
        let task_service = TaskService::new(db);
        task_service.create_task("Test Task", None, None, None).unwrap().id
    }

    // LinkService tests
    #[test]
    fn test_add_link_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = LinkService::new(&db);

        let link = service.add_link(task_id, "https://example.com", Some("Example")).unwrap();
        assert_eq!(link.url, "https://example.com");
        assert_eq!(link.title, "Example");
    }

    #[test]
    fn test_add_link_default_title() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = LinkService::new(&db);

        let link = service.add_link(task_id, "https://example.com", None).unwrap();
        assert_eq!(link.title, "https://example.com");
    }

    #[test]
    fn test_add_link_invalid_url() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = LinkService::new(&db);

        let result = service.add_link(task_id, "invalid-url", None);
        assert!(matches!(result, Err(TrackError::InvalidUrl(_))));
    }

    #[test]
    fn test_list_links() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = LinkService::new(&db);

        service.add_link(task_id, "https://example1.com", None).unwrap();
        service.add_link(task_id, "https://example2.com", None).unwrap();

        let links = service.list_links(task_id).unwrap();
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn test_validate_url_http() {
        let db = setup_db();
        let service = LinkService::new(&db);

        assert!(service.validate_url("http://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_https() {
        let db = setup_db();
        let service = LinkService::new(&db);

        assert!(service.validate_url("https://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        let db = setup_db();
        let service = LinkService::new(&db);

        assert!(matches!(
            service.validate_url("ftp://example.com"),
            Err(TrackError::InvalidUrl(_))
        ));
    }

    // ScrapService tests
    #[test]
    fn test_add_scrap_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = ScrapService::new(&db);

        let scrap = service.add_scrap(task_id, "Test scrap content").unwrap();
        assert_eq!(scrap.content, "Test scrap content");
    }

    #[test]
    fn test_get_scrap_success() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = ScrapService::new(&db);

        let created = service.add_scrap(task_id, "Test scrap").unwrap();
        let retrieved = service.get_scrap(created.id).unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.content, "Test scrap");
    }

    #[test]
    fn test_list_scraps() {
        let db = setup_db();
        let task_id = create_test_task(&db);
        let service = ScrapService::new(&db);

        service.add_scrap(task_id, "Scrap 1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        service.add_scrap(task_id, "Scrap 2").unwrap();

        let scraps = service.list_scraps(task_id).unwrap();
        assert_eq!(scraps.len(), 2);
        // Should be in descending order (newest first)
        assert_eq!(scraps[0].content, "Scrap 2");
        assert_eq!(scraps[1].content, "Scrap 1");
    }
}

