use crate::errors::Result;
use crate::models::UserContext;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone)]
pub struct UserContextStore {
    db_path: String,
}

impl UserContextStore {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_string_lossy().to_string();
        let store = Self { db_path };
        store.init_db()?;
        Ok(store)
    }

    fn init_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_context (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                context_key TEXT NOT NULL,
                context_value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(user_id, context_key)
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_context_user_id ON user_context(user_id)",
            [],
        )?;
        Ok(())
    }

    pub fn get_context(&self, user_id: &str) -> Result<UserContext> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT context_key, context_value FROM user_context WHERE user_id = ?"
        )?;
        
        let rows = stmt.query_map(params![user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut name_mappings = HashMap::new();
        let mut location_preferences = HashMap::new();
        let mut terminology = HashMap::new();
        let mut forbidden_words = Vec::new();
        let mut default_tone = "professional".to_string();
        let mut default_format = None;
        let created_at = chrono::Utc::now();
        let updated_at = chrono::Utc::now();

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "name_mappings" => {
                    name_mappings = serde_json::from_str(&value)?;
                }
                "location_preferences" => {
                    location_preferences = serde_json::from_str(&value)?;
                }
                "terminology" => {
                    terminology = serde_json::from_str(&value)?;
                }
                "forbidden_words" => {
                    forbidden_words = serde_json::from_str(&value)?;
                }
                "default_tone" => {
                    default_tone = serde_json::from_str(&value)?;
                }
                "default_format" => {
                    default_format = serde_json::from_str(&value)?;
                }
                _ => {}
            }
        }

        Ok(UserContext {
            user_id: user_id.to_string(),
            name_mappings,
            location_preferences,
            terminology,
            forbidden_words,
            default_tone,
            default_format,
            created_at,
            updated_at,
        })
    }

    pub fn update_context(&self, user_id: &str, context_key: &str, context_value: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let now = chrono::Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT OR REPLACE INTO user_context (user_id, context_key, context_value, created_at, updated_at)
             VALUES (?1, ?2, ?3, COALESCE((SELECT created_at FROM user_context WHERE user_id = ?1 AND context_key = ?2), ?4), ?4)",
            params![user_id, context_key, context_value, now],
        )?;
        
        Ok(())
    }

    pub fn save_user_context(&self, context: &UserContext) -> Result<()> {
        self.update_context(&context.user_id, "name_mappings", &serde_json::to_string(&context.name_mappings)?)?;
        self.update_context(&context.user_id, "location_preferences", &serde_json::to_string(&context.location_preferences)?)?;
        self.update_context(&context.user_id, "terminology", &serde_json::to_string(&context.terminology)?)?;
        self.update_context(&context.user_id, "forbidden_words", &serde_json::to_string(&context.forbidden_words)?)?;
        self.update_context(&context.user_id, "default_tone", &serde_json::to_string(&context.default_tone)?)?;
        if let Some(format) = &context.default_format {
            self.update_context(&context.user_id, "default_format", &serde_json::to_string(format)?)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_context(&self, user_id: &str, context_key: Option<&str>) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        
        if let Some(key) = context_key {
            conn.execute(
                "DELETE FROM user_context WHERE user_id = ? AND context_key = ?",
                params![user_id, key],
            )?;
        } else {
            conn.execute(
                "DELETE FROM user_context WHERE user_id = ?",
                params![user_id],
            )?;
        }
        
        Ok(())
    }

    #[allow(dead_code)]
    pub fn export_context(&self, user_id: &str, export_path: impl AsRef<Path>) -> Result<()> {
        let context = self.get_context(user_id)?;
        let json = serde_json::to_string_pretty(&context)?;
        std::fs::write(export_path, json)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn import_context(&self, user_id: &str, import_path: impl AsRef<Path>) -> Result<()> {
        let json = std::fs::read_to_string(import_path)?;
        let context: UserContext = serde_json::from_str(&json)?;
        
        // Save each field
        self.update_context(user_id, "name_mappings", &serde_json::to_string(&context.name_mappings)?)?;
        self.update_context(user_id, "location_preferences", &serde_json::to_string(&context.location_preferences)?)?;
        self.update_context(user_id, "terminology", &serde_json::to_string(&context.terminology)?)?;
        self.update_context(user_id, "forbidden_words", &serde_json::to_string(&context.forbidden_words)?)?;
        self.update_context(user_id, "default_tone", &serde_json::to_string(&context.default_tone)?)?;
        
        if let Some(format) = &context.default_format {
            self.update_context(user_id, "default_format", &serde_json::to_string(format)?)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_context() -> UserContext {
        let mut name_mappings = HashMap::new();
        name_mappings.insert("小李".to_string(), "李经理".to_string());
        let mut loc = HashMap::new();
        loc.insert("家".to_string(), "上海市浦东新区".to_string());
        let mut terms = HashMap::new();
        terms.insert("PPT".to_string(), "演示文稿".to_string());

        UserContext {
            user_id: "test_user".to_string(),
            name_mappings,
            location_preferences: loc,
            terminology: terms,
            forbidden_words: vec!["敏感词".to_string()],
            default_tone: "professional".to_string(),
            default_format: Some("email".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_save_and_load_context() {
        let db_path = "/tmp/aura_test_context.db";
        let _ = std::fs::remove_file(db_path);

        let store = UserContextStore::new(db_path).unwrap();
        let ctx = make_context();
        store.save_user_context(&ctx).unwrap();

        let loaded = store.get_context("test_user").unwrap();
        assert_eq!(loaded.name_mappings.get("小李").unwrap(), "李经理");
        assert_eq!(loaded.location_preferences.get("家").unwrap(), "上海市浦东新区");
        assert_eq!(loaded.default_tone, "professional");
        assert_eq!(loaded.default_format, Some("email".to_string()));
        assert_eq!(loaded.forbidden_words, vec!["敏感词".to_string()]);
    }

    #[test]
    fn test_get_context_returns_defaults_for_unknown_user() {
        let db_path = "/tmp/aura_test_context_new.db";
        let _ = std::fs::remove_file(db_path);
        let store = UserContextStore::new(db_path).unwrap();
        let ctx = store.get_context("nonexistent").unwrap();
        assert_eq!(ctx.default_tone, "professional");
        assert!(ctx.name_mappings.is_empty());
    }

    #[test]
    fn test_update_context() {
        let db_path = "/tmp/aura_test_update.db";
        let _ = std::fs::remove_file(db_path);
        let store = UserContextStore::new(db_path).unwrap();

        store.update_context("user1", "name_mappings", r#"{"苏":"苏总"}"#).unwrap();
        let ctx = store.get_context("user1").unwrap();
        assert_eq!(ctx.name_mappings.get("苏").unwrap(), "苏总");
    }
}
