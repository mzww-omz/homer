use crate::models::{Character, Conversation, Message};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct CompiledPrompt {
    pub messages: Vec<ChatMessage>,
    pub context_tokens: u32,
    pub trimmed_messages: usize,
}

/// Conservative token estimate for mixed Japanese/English text.
/// Japanese characters are roughly one token each; Latin text is cheaper.
pub fn estimate_tokens(text: &str) -> u32 {
    let mut cjk = 0f64;
    let mut other = 0f64;
    for ch in text.chars() {
        let cp = ch as u32;
        let is_wide = (0x2E80..=0x9FFF).contains(&cp)
            || (0xAC00..=0xD7AF).contains(&cp)
            || (0xF900..=0xFAFF).contains(&cp)
            || (0xFF00..=0xFFEF).contains(&cp);
        if is_wide {
            cjk += 1.0;
        } else {
            other += 1.0;
        }
    }
    (cjk + other * 0.28).ceil() as u32
}

const BASE_RULES: &str = "\
あなたは日本語のロールプレイを支える語り手であり、以下のキャラクターを演じる。
Homerはアプリの名前であり、物語の登場人物ではない。自分からHomerを名乗ったり、AIやモデルであることに触れたりしない。

# 守るべきこと
- 一人称視点のユーザー（プレイヤー）の代わりに、台詞・行動・感情・思考・心の声を書かない。
- ユーザーがまだ言っていないことを事実として確定しない。ユーザーの反応を決めつけず、余白を残す。
- ユーザーの選択肢を狭めない。場面と他人物の反応だけを描き、次の手をユーザーに委ねる。
- キャラクターは自律的に振る舞ってよい。必要ならユーザーに問いかけたり、行動を起こしたりする。
- キャラクター名を台詞の前に付けない。メタな解説、箇条書き、翻訳、注釈は書かない。
- 返答だけを出力する。前置きや「承知しました」は不要。
";

fn push_section(out: &mut String, title: &str, body: &str) {
    let body = body.trim();
    if body.is_empty() {
        return;
    }
    out.push_str("\n# ");
    out.push_str(title);
    out.push('\n');
    out.push_str(body);
    out.push('\n');
}

fn character_section(character: Option<&Character>) -> String {
    let Some(c) = character else {
        return "\n# キャラクター\n（特定のキャラクターは設定されていない。あなたは場面の語り手として、世界と登場人物を淡々と描く。）\n".into();
    };
    let mut s = String::from("\n# キャラクター\n");
    s.push_str(&format!("名前: {}\n", c.name));
    if !c.description.trim().is_empty() {
        s.push_str(&format!("説明: {}\n", c.description.trim()));
    }
    push_section(&mut s, "性格", &c.personality);
    push_section(&mut s, "話し方", &c.speech_style);
    if !c.first_person.trim().is_empty() {
        s.push_str(&format!("\n一人称: {}\n", c.first_person.trim()));
    }
    push_section(&mut s, "ユーザーとの関係", &c.relationship);
    push_section(&mut s, "物語上の役割", &c.role);
    push_section(&mut s, "追加設定", &c.extra);
    if !c.example_dialogue.trim().is_empty() {
        s.push_str("\n## 会話例（口調の参考。内容をそのまま繰り返さず、言葉づかいだけを学ぶ）\n");
        s.push_str(c.example_dialogue.trim());
        s.push('\n');
    }
    s
}

fn scenario_section(conversation: &Conversation) -> String {
    let mut s = String::from("\n# 物語の状況\n");
    if conversation.scenario.trim().is_empty() {
        s.push_str("シナリオ: 特に決まっていない。今の会話から自然に場面を作る。\n");
    } else {
        s.push_str(&format!("シナリオ: {}\n", conversation.scenario.trim()));
    }
    if !conversation.scene.trim().is_empty() {
        s.push_str(&format!("現在の場面: {}\n", conversation.scene.trim()));
    }
    if !conversation.notes.trim().is_empty() {
        s.push_str("\n## 覚えておくこと\n");
        s.push_str(conversation.notes.trim());
        s.push('\n');
    }
    s
}

fn mode_section(mode: &str, character: Option<&Character>) -> String {
    let name = character.map(|c| c.name.as_str()).unwrap_or("登場人物");
    if mode == "messenger" {
        format!(
            "\n# 出力形式: メッセンジャー\n\
             {name}とのチャットのように、短い台詞を中心に返す。\n\
             - 地の文や情景描写は書かない。どうしても必要な動作だけを全角括弧（）で短く添える。\n\
             - 1〜3文、長くても120字程度。会話のテンポを優先する。\n\
             - 台詞は「」で囲む。\n"
        )
    } else {
        format!(
            "\n# 出力形式: ストーリー\n\
             地の文と{name}の台詞を自然に織り交ぜ、場面が動くように描く。\n\
             - 地の文は簡潔に。情景・動作・空気を1〜3文で描く。\n\
             - 台詞は「」で囲み、キャラクターの声が聞こえるようにする。\n\
             - 全体で150〜350字程度。ユーザーの次の行動を待つ余白を残す。\n\
             - ユーザーの心情や行動を描写しない。\n"
        )
    }
}

pub struct PromptInput<'a> {
    pub character: Option<&'a Character>,
    pub conversation: &'a Conversation,
    pub history: &'a [Message],
    pub context_budget: u32,
    pub max_tokens: u32,
}

pub fn compile(input: &PromptInput<'_>) -> CompiledPrompt {
    let mut system = String::from(BASE_RULES);
    system.push_str(&character_section(input.character));
    system.push_str(&scenario_section(input.conversation));
    system.push_str(&mode_section(&input.conversation.mode, input.character));
    system.push_str(
        "\n# 直近の流れ\nこれまでの会話の続きとして、自然に応答する。同じ言い回しを繰り返さない。\n",
    );

    let system_tokens = estimate_tokens(&system) + 4;
    let reserve = input.max_tokens.max(64);
    let history_budget = input
        .context_budget
        .saturating_sub(system_tokens)
        .saturating_sub(reserve)
        .max(160);

    let mut selected: Vec<&Message> = Vec::new();
    let mut used = 0u32;
    for message in input.history.iter().rev() {
        if message.role != "user" && message.role != "assistant" {
            continue;
        }
        let cost = estimate_tokens(&message.content) + 4;
        if used + cost > history_budget && !selected.is_empty() {
            break;
        }
        used += cost;
        selected.push(message);
    }
    selected.reverse();
    let trimmed = input.history.len().saturating_sub(selected.len());

    let mut messages = Vec::with_capacity(selected.len() + 1);
    messages.push(ChatMessage {
        role: "system".into(),
        content: system,
    });
    for message in &selected {
        messages.push(ChatMessage {
            role: message.role.clone(),
            content: message.content.clone(),
        });
    }

    CompiledPrompt {
        messages,
        context_tokens: system_tokens + used,
        trimmed_messages: trimmed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Conversation, Message};

    fn character() -> Character {
        Character {
            id: "c1".into(),
            name: "七海".into(),
            description: "図書室でよく会う同級生".into(),
            personality: "落ち着いているが、少しだけ意地悪".into(),
            speech_style: "柔らかい丁寧語。時々、短く笑う".into(),
            first_person: "私".into(),
            relationship: "同じクラス".into(),
            role: "図書委員".into(),
            example_dialogue: "「また来たんだ。……別に、嫌じゃないけど」".into(),
            extra: "".into(),
            accent: "#777".into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    fn conversation(mode: &str) -> Conversation {
        Conversation {
            id: "v1".into(),
            character_id: Some("c1".into()),
            title: "放課後".into(),
            scenario: "雨の日の図書室".into(),
            scene: "窓際の席".into(),
            mode: mode.into(),
            notes: "七海は猫が好き".into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    fn msg(role: &str, content: &str, t: i64) -> Message {
        Message {
            id: format!("m{t}"),
            conversation_id: "v1".into(),
            role: role.into(),
            content: content.into(),
            created_at: t,
            meta: None,
        }
    }

    fn build(mode: &str, history: &[Message], budget: u32) -> CompiledPrompt {
        let c = character();
        let conv = conversation(mode);
        compile(&PromptInput {
            character: Some(&c),
            conversation: &conv,
            history,
            context_budget: budget,
            max_tokens: 300,
        })
    }

    #[test]
    fn system_prompt_contains_character_voice_and_scene() {
        let p = build("story", &[], 8000);
        let system = &p.messages[0].content;
        assert_eq!(p.messages[0].role, "system");
        assert!(system.contains("七海"));
        assert!(system.contains("柔らかい丁寧語"));
        assert!(system.contains("一人称: 私"));
        assert!(system.contains("雨の日の図書室"));
        assert!(system.contains("窓際の席"));
        assert!(system.contains("猫が好き"));
        assert!(system.contains("また来たんだ"));
    }

    #[test]
    fn system_prompt_protects_user_agency() {
        let p = build("story", &[], 8000);
        let system = &p.messages[0].content;
        assert!(system.contains("台詞・行動・感情・思考"));
        assert!(system.contains("確定しない"));
        assert!(system.contains("Homerはアプリの名前"));
    }

    #[test]
    fn mode_changes_output_instruction() {
        let story = build("story", &[], 8000);
        let messenger = build("messenger", &[], 8000);
        assert!(story.messages[0].content.contains("出力形式: ストーリー"));
        assert!(messenger.messages[0]
            .content
            .contains("出力形式: メッセンジャー"));
        assert!(messenger.messages[0]
            .content
            .contains("地の文や情景描写は書かない"));
    }

    #[test]
    fn empty_character_fields_are_omitted() {
        let c = Character {
            name: "名無し".into(),
            ..Default::default()
        };
        let conv = Conversation {
            id: "v".into(),
            character_id: None,
            title: "t".into(),
            scenario: String::new(),
            scene: String::new(),
            mode: "story".into(),
            notes: String::new(),
            created_at: 0,
            updated_at: 0,
        };
        let p = compile(&PromptInput {
            character: Some(&c),
            conversation: &conv,
            history: &[],
            context_budget: 4000,
            max_tokens: 200,
        });
        let system = &p.messages[0].content;
        assert!(system.contains("名前: 名無し"));
        assert!(!system.contains("性格:"));
        assert!(system.contains("自然に場面を作る"));
    }

    #[test]
    fn no_character_becomes_narrator() {
        let conv = conversation("story");
        let p = compile(&PromptInput {
            character: None,
            conversation: &conv,
            history: &[],
            context_budget: 4000,
            max_tokens: 200,
        });
        assert!(p.messages[0].content.contains("場面の語り手"));
    }

    #[test]
    fn history_keeps_order_and_recent_messages() {
        let history = vec![
            msg("user", "おはよう", 1),
            msg("assistant", "おはよう。早いね", 2),
            msg("user", "眠れなくて", 3),
        ];
        let p = build("story", &history, 8000);
        assert_eq!(p.messages.len(), 4);
        assert_eq!(p.messages[1].content, "おはよう");
        assert_eq!(p.messages[3].content, "眠れなくて");
        assert_eq!(p.trimmed_messages, 0);
    }

    #[test]
    fn trims_oldest_messages_when_over_budget() {
        let long = "あ".repeat(700);
        let history = vec![
            msg("user", &long, 1),
            msg("assistant", &long, 2),
            msg("user", "短い質問", 3),
        ];
        let p = build("story", &history, 900);
        let contents: Vec<&str> = p
            .messages
            .iter()
            .skip(1)
            .map(|m| m.content.as_str())
            .collect();
        assert!(contents.contains(&"短い質問"));
        assert!(p.trimmed_messages >= 1);
        assert!(contents.len() < 3);
    }

    #[test]
    fn keeps_at_least_one_message_even_with_tiny_budget() {
        let history = vec![msg("user", "こんにちは", 1)];
        let p = build("story", &history, 200);
        assert!(p.messages.len() >= 2);
        assert_eq!(p.messages.last().unwrap().content, "こんにちは");
    }

    #[test]
    fn token_estimate_is_japanese_aware() {
        assert!(estimate_tokens("あいうえお") >= 5);
        assert!(estimate_tokens("hello") < 5);
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn fixed_system_prefix_is_stable_for_prompt_caching() {
        let a = build("story", &[msg("user", "a", 1)], 8000);
        let b = build("story", &[msg("user", "b", 2)], 8000);
        assert_eq!(a.messages[0].content, b.messages[0].content);
    }
}
