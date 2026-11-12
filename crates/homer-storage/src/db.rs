use homer_core::models::*;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub type DbResult<T> = Result<T, String>;

const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE characters (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        personality TEXT NOT NULL DEFAULT '',
        speech_style TEXT NOT NULL DEFAULT '',
        first_person TEXT NOT NULL DEFAULT '',
        relationship TEXT NOT NULL DEFAULT '',
        role TEXT NOT NULL DEFAULT '',
        example_dialogue TEXT NOT NULL DEFAULT '',
        extra TEXT NOT NULL DEFAULT '',
        accent TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE conversations (
        id TEXT PRIMARY KEY,
        character_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
        title TEXT NOT NULL,
        scenario TEXT NOT NULL DEFAULT '',
        scene TEXT NOT NULL DEFAULT '',
        mode TEXT NOT NULL DEFAULT 'story',
        notes TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE messages (
        id TEXT PRIMARY KEY,
        conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        meta TEXT
    );

    CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at);

    CREATE TABLE settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
];

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

pub fn open_db(path: &Path) -> DbResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("保存フォルダを作成できません: {e}"))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("データベースを開けません: {e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("WALを設定できません: {e}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("外部キーを設定できません: {e}"))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("busy_timeoutを設定できません: {e}"))?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn migrate(conn: &Connection) -> DbResult<()> {
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| format!("スキーマ版を取得できません: {e}"))?;
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let target = (index + 1) as i64;
        if version < target {
            conn.execute_batch(sql)
                .map_err(|e| format!("マイグレーション v{target} に失敗しました: {e}"))?;
            conn.pragma_update(None, "user_version", target)
                .map_err(|e| format!("スキーマ版を更新できません: {e}"))?;
        }
    }
    Ok(())
}

// ---------- characters ----------

fn map_character(row: &rusqlite::Row<'_>) -> rusqlite::Result<Character> {
    Ok(Character {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        personality: row.get(3)?,
        speech_style: row.get(4)?,
        first_person: row.get(5)?,
        relationship: row.get(6)?,
        role: row.get(7)?,
        example_dialogue: row.get(8)?,
        extra: row.get(9)?,
        accent: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const CHAR_COLS: &str = "id, name, description, personality, speech_style, first_person, relationship, role, example_dialogue, extra, accent, created_at, updated_at";

pub fn list_characters(conn: &Connection) -> DbResult<Vec<Character>> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {CHAR_COLS} FROM characters ORDER BY updated_at DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_character)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn get_character(conn: &Connection, id: &str) -> DbResult<Character> {
    conn.query_row(
        &format!("SELECT {CHAR_COLS} FROM characters WHERE id = ?1"),
        params![id],
        map_character,
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "キャラクターが見つかりません".to_string())
}

pub fn create_character(conn: &Connection, input: &CharacterInput) -> DbResult<Character> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("名前を入力してください".into());
    }
    let now = now_ms();
    let character = Character {
        id: new_id("char"),
        name: name.to_string(),
        description: input.description.trim().to_string(),
        personality: input.personality.trim().to_string(),
        speech_style: input.speech_style.trim().to_string(),
        first_person: input.first_person.trim().to_string(),
        relationship: input.relationship.trim().to_string(),
        role: input.role.trim().to_string(),
        example_dialogue: input.example_dialogue.trim().to_string(),
        extra: input.extra.trim().to_string(),
        accent: if input.accent.trim().is_empty() {
            "#7c7468".into()
        } else {
            input.accent.trim().to_string()
        },
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        &format!(
            "INSERT INTO characters ({CHAR_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"
        ),
        params![
            character.id,
            character.name,
            character.description,
            character.personality,
            character.speech_style,
            character.first_person,
            character.relationship,
            character.role,
            character.example_dialogue,
            character.extra,
            character.accent,
            character.created_at,
            character.updated_at
        ],
    )
    .map_err(|e| format!("キャラクターを保存できません: {e}"))?;
    Ok(character)
}

pub fn update_character(
    conn: &Connection,
    id: &str,
    input: &CharacterInput,
) -> DbResult<Character> {
    let existing = get_character(conn, id)?;
    let name = input.name.trim();
    if name.is_empty() {
        return Err("名前を入力してください".into());
    }
    let now = now_ms();
    let accent = if input.accent.trim().is_empty() {
        existing.accent
    } else {
        input.accent.trim().to_string()
    };
    conn.execute(
        "UPDATE characters SET name=?2, description=?3, personality=?4, speech_style=?5,
         first_person=?6, relationship=?7, role=?8, example_dialogue=?9, extra=?10,
         accent=?11, updated_at=?12 WHERE id=?1",
        params![
            id,
            name,
            input.description.trim(),
            input.personality.trim(),
            input.speech_style.trim(),
            input.first_person.trim(),
            input.relationship.trim(),
            input.role.trim(),
            input.example_dialogue.trim(),
            input.extra.trim(),
            accent,
            now
        ],
    )
    .map_err(|e| format!("キャラクターを更新できません: {e}"))?;
    get_character(conn, id)
}

pub fn delete_character(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM characters WHERE id=?1", params![id])
        .map_err(|e| format!("キャラクターを削除できません: {e}"))?;
    Ok(())
}

// ---------- conversations ----------

fn map_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        character_id: row.get(1)?,
        title: row.get(2)?,
        scenario: row.get(3)?,
        scene: row.get(4)?,
        mode: row.get(5)?,
        notes: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const CONV_COLS: &str =
    "id, character_id, title, scenario, scene, mode, notes, created_at, updated_at";

pub fn list_conversations(conn: &Connection) -> DbResult<Vec<Conversation>> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {CONV_COLS} FROM conversations ORDER BY updated_at DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_conversation)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn get_conversation(conn: &Connection, id: &str) -> DbResult<Conversation> {
    conn.query_row(
        &format!("SELECT {CONV_COLS} FROM conversations WHERE id=?1"),
        params![id],
        map_conversation,
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "会話が見つかりません".to_string())
}

pub fn create_conversation(conn: &Connection, input: &ConversationInput) -> DbResult<Conversation> {
    let now = now_ms();
    let character_name = match input.character_id.as_deref() {
        Some(id) if !id.is_empty() => get_character(conn, id).ok().map(|c| c.name),
        _ => None,
    };
    let title = if input.title.trim().is_empty() {
        match &character_name {
            Some(name) => format!("{name}との物語"),
            None => "新しい物語".to_string(),
        }
    } else {
        input.title.trim().to_string()
    };
    let mode = match input.mode.as_deref() {
        Some("messenger") => "messenger",
        _ => "story",
    };
    let conversation = Conversation {
        id: new_id("conv"),
        character_id: input.character_id.clone().filter(|s| !s.is_empty()),
        title,
        scenario: input.scenario.trim().to_string(),
        scene: input.scene.trim().to_string(),
        mode: mode.to_string(),
        notes: input.notes.trim().to_string(),
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        &format!("INSERT INTO conversations ({CONV_COLS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)"),
        params![
            conversation.id,
            conversation.character_id,
            conversation.title,
            conversation.scenario,
            conversation.scene,
            conversation.mode,
            conversation.notes,
            conversation.created_at,
            conversation.updated_at
        ],
    )
    .map_err(|e| format!("会話を作成できません: {e}"))?;
    Ok(conversation)
}

pub fn update_conversation(
    conn: &Connection,
    id: &str,
    patch: &ConversationUpdate,
) -> DbResult<Conversation> {
    let mut conv = get_conversation(conn, id)?;
    if let Some(title) = &patch.title {
        if !title.trim().is_empty() {
            conv.title = title.trim().to_string();
        }
    }
    if let Some(v) = &patch.scenario {
        conv.scenario = v.trim().to_string();
    }
    if let Some(v) = &patch.scene {
        conv.scene = v.trim().to_string();
    }
    if let Some(v) = &patch.notes {
        conv.notes = v.trim().to_string();
    }
    if let Some(v) = &patch.mode {
        if v == "story" || v == "messenger" {
            conv.mode = v.clone();
        }
    }
    conv.updated_at = now_ms();
    conn.execute(
        "UPDATE conversations SET title=?2, scenario=?3, scene=?4, mode=?5, notes=?6, updated_at=?7 WHERE id=?1",
        params![
            id,
            conv.title,
            conv.scenario,
            conv.scene,
            conv.mode,
            conv.notes,
            conv.updated_at
        ],
    )
    .map_err(|e| format!("会話を更新できません: {e}"))?;
    Ok(conv)
}

pub fn touch_conversation(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute(
        "UPDATE conversations SET updated_at=?2 WHERE id=?1",
        params![id, now_ms()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_conversation(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM conversations WHERE id=?1", params![id])
        .map_err(|e| format!("会話を削除できません: {e}"))?;
    Ok(())
}

// ---------- messages ----------

fn map_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    let meta: Option<String> = row.get(5)?;
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        meta: meta.and_then(|raw| serde_json::from_str::<MessageMeta>(&raw).ok()),
    })
}

const MSG_COLS: &str = "id, conversation_id, role, content, created_at, meta";

pub fn list_messages(conn: &Connection, conversation_id: &str) -> DbResult<Vec<Message>> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {MSG_COLS} FROM messages WHERE conversation_id=?1 ORDER BY created_at ASC, rowid ASC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![conversation_id], map_message)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn insert_message(
    conn: &Connection,
    conversation_id: &str,
    role: &str,
    content: &str,
    meta: Option<&MessageMeta>,
) -> DbResult<Message> {
    let message = Message {
        id: new_id("msg"),
        conversation_id: conversation_id.to_string(),
        role: role.to_string(),
        content: content.to_string(),
        created_at: now_ms(),
        meta: meta.cloned(),
    };
    let meta_raw = message
        .meta
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());
    conn.execute(
        &format!("INSERT INTO messages ({MSG_COLS}) VALUES (?1,?2,?3,?4,?5,?6)"),
        params![
            message.id,
            message.conversation_id,
            message.role,
            message.content,
            message.created_at,
            meta_raw
        ],
    )
    .map_err(|e| format!("メッセージを保存できません: {e}"))?;
    touch_conversation(conn, conversation_id)?;
    Ok(message)
}

#[allow(dead_code)]
pub fn delete_message(conn: &Connection, id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM messages WHERE id=?1", params![id])
        .map_err(|e| format!("メッセージを削除できません: {e}"))?;
    Ok(())
}

// ---------- settings ----------

pub fn load_settings(conn: &Connection) -> DbResult<Settings> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key='settings'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some(raw) = raw else {
        return Ok(Settings::default());
    };
    // Merge with defaults so older settings survive new fields.
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let mut merged = serde_json::to_value(Settings::default()).unwrap_or_default();
    if let (Some(base), Some(overlay)) = (merged.as_object_mut(), value.as_object()) {
        for (k, v) in overlay {
            base.insert(k.clone(), v.clone());
        }
    }
    serde_json::from_value(merged).map_err(|e| format!("設定を読み込めません: {e}"))
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> DbResult<()> {
    let raw = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('settings', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![raw],
    )
    .map_err(|e| format!("設定を保存できません: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn sample_character() -> CharacterInput {
        CharacterInput {
            name: "七海".into(),
            description: "放課後の図書室でよく会う同級生".into(),
            speech_style: "落ち着いた口調".into(),
            ..Default::default()
        }
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = memory_db();
        migrate(&conn).unwrap();
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn character_roundtrip() {
        let conn = memory_db();
        let created = create_character(&conn, &sample_character()).unwrap();
        assert_eq!(created.name, "七海");
        let list = list_characters(&conn).unwrap();
        assert_eq!(list.len(), 1);
        let mut update = sample_character();
        update.speech_style = "少しだけ砕けた口調".into();
        let updated = update_character(&conn, &created.id, &update).unwrap();
        assert_eq!(updated.speech_style, "少しだけ砕けた口調");
        delete_character(&conn, &created.id).unwrap();
        assert!(list_characters(&conn).unwrap().is_empty());
    }

    #[test]
    fn character_requires_name() {
        let conn = memory_db();
        let mut input = sample_character();
        input.name = "   ".into();
        assert!(create_character(&conn, &input).is_err());
    }

    #[test]
    fn conversation_and_message_persist_and_cascade() {
        let conn = memory_db();
        let character = create_character(&conn, &sample_character()).unwrap();
        let conv = create_conversation(
            &conn,
            &ConversationInput {
                character_id: Some(character.id.clone()),
                scenario: "図書室".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(conv.title, "七海との物語");
        let msg = insert_message(&conn, &conv.id, "user", "こんにちは", None).unwrap();
        let loaded = list_messages(&conn, &conv.id).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "こんにちは");
        assert_eq!(loaded[0].id, msg.id);

        delete_conversation(&conn, &conv.id).unwrap();
        assert!(list_messages(&conn, &conv.id).unwrap().is_empty());
        assert_eq!(list_characters(&conn).unwrap().len(), 1);
    }

    #[test]
    fn conversation_update_keeps_data() {
        let conn = memory_db();
        let conv = create_conversation(
            &conn,
            &ConversationInput {
                title: "放課後".into(),
                scenario: "雨の図書室".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let updated = update_conversation(
            &conn,
            &conv.id,
            &ConversationUpdate {
                title: None,
                scenario: None,
                scene: Some("窓際の席".into()),
                mode: Some("messenger".into()),
                notes: Some("七海は猫が好き".into()),
            },
        )
        .unwrap();
        assert_eq!(updated.title, "放課後");
        assert_eq!(updated.scenario, "雨の図書室");
        assert_eq!(updated.scene, "窓際の席");
        assert_eq!(updated.mode, "messenger");
        assert_eq!(updated.notes, "七海は猫が好き");
    }

    #[test]
    fn settings_merge_with_defaults() {
        let conn = memory_db();
        let defaults = load_settings(&conn).unwrap();
        assert_eq!(defaults.model, "");
        let mut settings = defaults.clone();
        settings.model = "test-model".into();
        settings.max_tokens = 123;
        save_settings(&conn, &settings).unwrap();
        let loaded = load_settings(&conn).unwrap();
        assert_eq!(loaded.model, "test-model");
        assert_eq!(loaded.max_tokens, 123);
        assert_eq!(loaded.base_url, defaults.base_url);
    }

    #[test]
    fn meta_json_roundtrip() {
        let conn = memory_db();
        let conv = create_conversation(&conn, &ConversationInput::default()).unwrap();
        let meta = MessageMeta {
            metrics: Some(GenMetrics {
                ttft_ms: 10,
                ..Default::default()
            }),
            stopped: true,
            incomplete: false,
        };
        insert_message(&conn, &conv.id, "assistant", "やあ", Some(&meta)).unwrap();
        let loaded = list_messages(&conn, &conv.id).unwrap();
        let loaded_meta = loaded[0].meta.as_ref().unwrap();
        assert_eq!(loaded_meta.metrics.as_ref().unwrap().ttft_ms, 10);
        assert!(loaded_meta.stopped);
    }
}
