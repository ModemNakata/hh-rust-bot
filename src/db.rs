use rusqlite::{Connection, Result as SqlResult};
use std::sync::Mutex;

/// Database manager for tracking subscribers and config
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Initialize database connection and create tables
    pub fn new(db_path: &str) -> SqlResult<Self> {
        println!("[DB_INIT] Initializing database at: {}", db_path);
        
        let conn = Connection::open(db_path)?;
        
        println!("[DB_INIT] Connection established");
        
        // Create subscribers table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscribers (
                id INTEGER PRIMARY KEY,
                chat_id INTEGER UNIQUE NOT NULL,
                chat_type TEXT,
                first_seen DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        println!("[DB_INIT] Subscribers table ready");
        
        // Create config table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )?;
        println!("[DB_INIT] Config table ready");
        
        // Initialize latest_vacancy_id if not exists
        conn.execute(
            "INSERT OR IGNORE INTO config (key, value) VALUES ('latest_vacancy_id', '0')",
            [],
        )?;
        println!("[DB_INIT] Latest vacancy ID initialized");
        
        println!("[DB_INIT] Database initialization complete");
        
        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    /// Add a chat_id to subscribers
    pub fn add_chat_id(&self, chat_id: i64, chat_type: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
        println!("[DB_QUERY] Adding chat_id: {}, type: {}", chat_id, chat_type);
        
        let result = conn.execute(
            "INSERT OR IGNORE INTO subscribers (chat_id, chat_type) VALUES (?1, ?2)",
            [chat_id.to_string(), chat_type.to_string()],
        );
        
        match &result {
            Ok(rows) => println!("[DB_QUERY] ✓ Added/verified {} row(s) for chat_id: {}", rows, chat_id),
            Err(e) => println!("[DB_ERROR] Failed to add chat_id {}: {}", chat_id, e),
        }
        
        result.map(|_| ())
    }

    /// Get all registered chat_ids
    pub fn get_all_chat_ids(&self) -> SqlResult<Vec<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        
        println!("[DB_QUERY] Fetching all registered chat_ids");
        
        let mut stmt = conn.prepare(
            "SELECT chat_id, chat_type FROM subscribers ORDER BY first_seen DESC"
        )?;
        
        let chat_ids = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<SqlResult<Vec<_>>>()?;
        
        println!("[DB_QUERY] ✓ Found {} subscriber(s)", chat_ids.len());
        for (chat_id, chat_type) in &chat_ids {
            println!("[DB_QUERY]   - chat_id: {}, type: {}", chat_id, chat_type);
        }
        
        Ok(chat_ids)
    }

    /// Get count of subscribers
    pub fn get_subscriber_count(&self) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM subscribers",
            [],
            |row| row.get(0),
        )?;
        
        println!("[DB_QUERY] Subscriber count: {}", count);
        
        Ok(count)
    }

    /// Get the latest vacancy ID
    pub fn get_latest_vacancy_id(&self) -> SqlResult<String> {
        let conn = self.conn.lock().unwrap();
        
        println!("[DB_QUERY] Fetching latest_vacancy_id from config");
        
        let value: String = conn.query_row(
            "SELECT value FROM config WHERE key = 'latest_vacancy_id'",
            [],
            |row| row.get(0),
        )?;
        
        println!("[DB_QUERY] ✓ Latest vacancy ID: {}", value);
        
        Ok(value)
    }

    /// Update the latest vacancy ID
    pub fn update_latest_vacancy_id(&self, vacancy_id: &str) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
        println!("[DB_QUERY] Updating latest_vacancy_id to: {}", vacancy_id);
        
        let result = conn.execute(
            "UPDATE config SET value = ?1 WHERE key = 'latest_vacancy_id'",
            [vacancy_id],
        );
        
        match &result {
            Ok(rows) => println!("[DB_QUERY] ✓ Updated {} row(s) with latest_vacancy_id: {}", rows, vacancy_id),
            Err(e) => println!("[DB_ERROR] Failed to update latest_vacancy_id: {}", e),
        }
        
        result.map(|_| ())
    }

    /// Check if a chat_id is already registered
    pub fn is_chat_registered(&self, chat_id: i64) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        
        println!("[DB_QUERY] Checking if chat_id {} is registered", chat_id);
        
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM subscribers WHERE chat_id = ?1",
            [chat_id.to_string()],
            |row| row.get(0),
        )?;
        
        let result = exists > 0;
        println!("[DB_QUERY] ✓ Chat_id {} registered: {}", chat_id, result);
        
        Ok(result)
    }

    /// Get all config values
    pub fn get_all_config(&self) -> SqlResult<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        
        println!("[DB_QUERY] Fetching all config values");
        
        let mut stmt = conn.prepare("SELECT key, value FROM config")?;
        
        let config = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<SqlResult<Vec<_>>>()?;
        
        println!("[DB_QUERY] ✓ Found {} config value(s)", config.len());
        for (key, value) in &config {
            println!("[DB_QUERY]   - {}: {}", key, value);
        }
        
        Ok(config)
    }
}
