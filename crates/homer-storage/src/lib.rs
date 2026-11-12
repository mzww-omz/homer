//! SQLite persistence for Homer. All methods are synchronous and cheap; the UI
//! calls them from a background task or during startup.

pub mod db;

use homer_core::models::*;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub use db::now_ms;

pub struct Storage {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let conn = db::open_db(&path)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| e.to_string())?;
        db::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn
            .lock()
            .map_err(|_| "データベースのロックに失敗しました".to_string())
    }

    pub fn list_characters(&self) -> Result<Vec<Character>, String> {
        db::list_characters(&*self.conn()?)
    }

    pub fn create_character(&self, input: &CharacterInput) -> Result<Character, String> {
        db::create_character(&*self.conn()?, input)
    }

    pub fn update_character(&self, id: &str, input: &CharacterInput) -> Result<Character, String> {
        db::update_character(&*self.conn()?, id, input)
    }

    pub fn delete_character(&self, id: &str) -> Result<(), String> {
        db::delete_character(&*self.conn()?, id)
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, String> {
        db::list_conversations(&*self.conn()?)
    }

    pub fn create_conversation(&self, input: &ConversationInput) -> Result<Conversation, String> {
        db::create_conversation(&*self.conn()?, input)
    }

    pub fn update_conversation(
        &self,
        id: &str,
        patch: &ConversationUpdate,
    ) -> Result<Conversation, String> {
        db::update_conversation(&*self.conn()?, id, patch)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), String> {
        db::delete_conversation(&*self.conn()?, id)
    }

    pub fn list_messages(&self, conversation_id: &str) -> Result<Vec<Message>, String> {
        db::list_messages(&*self.conn()?, conversation_id)
    }

    pub fn insert_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        meta: Option<&MessageMeta>,
    ) -> Result<Message, String> {
        db::insert_message(&*self.conn()?, conversation_id, role, content, meta)
    }

    pub fn load_settings(&self) -> Result<Settings, String> {
        db::load_settings(&*self.conn()?)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        db::save_settings(&*self.conn()?, settings)
    }

    pub fn character(&self, id: &str) -> Result<Character, String> {
        db::get_character(&*self.conn()?, id)
    }

    pub fn conversation(&self, id: &str) -> Result<Conversation, String> {
        db::get_conversation(&*self.conn()?, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character_input() -> CharacterInput {
        CharacterInput {
            name: "七海".into(),
            description: "図書室で会う同級生".into(),
            speech_style: "落ち着いた丁寧語".into(),
            accent: "#7d6f5c".into(),
            ..Default::default()
        }
    }

    #[test]
    fn data_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("homer.db");

        let character_id;
        let conversation_id;
        {
            let storage = Storage::open(&path).unwrap();
            let character = storage.create_character(&character_input()).unwrap();
            character_id = character.id.clone();
            let conversation = storage
                .create_conversation(&ConversationInput {
                    character_id: Some(character.id),
                    scenario: "雨の図書室".into(),
                    ..Default::default()
                })
                .unwrap();
            conversation_id = conversation.id.clone();
            storage
                .insert_message(&conversation.id, "user", "こんにちは", None)
                .unwrap();
            storage
                .insert_message(
                    &conversation.id,
                    "assistant",
                    "「……こんにちは」",
                    Some(&MessageMeta::default()),
                )
                .unwrap();
        }

        let reopened = Storage::open(&path).unwrap();
        assert_eq!(reopened.list_characters().unwrap().len(), 1);
        let conversations = reopened.list_conversations().unwrap();
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].id, conversation_id);
        assert_eq!(conversations[0].scenario, "雨の図書室");
        let messages = reopened.list_messages(&conversation_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "こんにちは");
        assert_eq!(reopened.character(&character_id).unwrap().name, "七海");
    }

    #[test]
    fn settings_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("homer.db");
        {
            let storage = Storage::open(&path).unwrap();
            let mut settings = storage.load_settings().unwrap();
            settings.model = "qwen2.5-7b-instruct".into();
            storage.save_settings(&settings).unwrap();
        }
        let storage = Storage::open(&path).unwrap();
        assert_eq!(
            storage.load_settings().unwrap().model,
            "qwen2.5-7b-instruct"
        );
    }

    #[test]
    fn in_memory_storage_works() {
        let storage = Storage::open_in_memory().unwrap();
        let character = storage.create_character(&character_input()).unwrap();
        assert_eq!(character.name, "七海");
    }
}
