//! Split model output into spoken lines and narration for display.
//! The original text is preserved; this is purely a presentation concern.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentType {
    Narration,
    Dialogue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentType,
    pub text: String,
}

fn quote_pair(ch: char) -> Option<char> {
    match ch {
        '「' => Some('」'),
        '『' => Some('』'),
        '“' => Some('”'),
        _ => None,
    }
}

fn push_segment(out: &mut Vec<Segment>, kind: SegmentType, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.kind == SegmentType::Dialogue && kind == SegmentType::Dialogue {
            last.text.push('\n');
            last.text.push_str(trimmed);
            return;
        }
    }
    out.push(Segment {
        kind,
        text: trimmed.to_string(),
    });
}

fn name_dialogue(line: &str, character_name: Option<&str>) -> Option<String> {
    let name = character_name?.trim();
    if name.is_empty() {
        return None;
    }
    let trimmed = line.trim().trim_start_matches(['-', '—', ' ']);
    let rest = trimmed.strip_prefix(name)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('：').or_else(|| rest.strip_prefix(':'))?;
    let rest = rest.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn is_unclosed_quote(line: &str) -> bool {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let Some(close) = quote_pair(first) else {
        return false;
    };
    !chars.as_str().contains(close)
}

pub fn segment_text(raw: &str, character_name: Option<&str>) -> Vec<Segment> {
    let text = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut segments = Vec::new();
    let mut narration = String::new();

    let flush_narration = |narration: &mut String, segments: &mut Vec<Segment>| {
        if narration.trim().is_empty() {
            narration.clear();
            return;
        }
        for block in narration.split("\n\n") {
            let mut pending = String::new();
            for line in block.split('\n') {
                if let Some(dialogue) = name_dialogue(line, character_name) {
                    if !pending.trim().is_empty() {
                        push_segment(segments, SegmentType::Narration, &pending);
                    }
                    pending.clear();
                    push_segment(segments, SegmentType::Dialogue, &dialogue);
                } else if is_unclosed_quote(line) {
                    if !pending.trim().is_empty() {
                        push_segment(segments, SegmentType::Narration, &pending);
                    }
                    pending.clear();
                    push_segment(
                        segments,
                        SegmentType::Dialogue,
                        line.trim().trim_start_matches(['-', '—', ' ']),
                    );
                } else {
                    if !pending.is_empty() {
                        pending.push('\n');
                    }
                    pending.push_str(line);
                }
            }
            if !pending.trim().is_empty() {
                push_segment(segments, SegmentType::Narration, &pending);
            }
        }
        narration.clear();
    };

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if let Some(close) = quote_pair(ch) {
            if let Some(offset) = chars[i + 1..].iter().position(|c| *c == close) {
                let end = i + 1 + offset;
                flush_narration(&mut narration, &mut segments);
                let dialogue: String = chars[i..=end].iter().collect();
                push_segment(&mut segments, SegmentType::Dialogue, &dialogue);
                i = end + 1;
                continue;
            }
        }
        narration.push(ch);
        i += 1;
    }
    flush_narration(&mut narration, &mut segments);
    segments
}

/// Messenger mode hides narration unless the message has no dialogue at all.
pub fn visible_segments(segments: &[Segment], mode: &str) -> Vec<Segment> {
    if mode != "messenger" {
        return segments.to_vec();
    }
    let dialogue: Vec<Segment> = segments
        .iter()
        .filter(|s| s.kind == SegmentType::Dialogue)
        .cloned()
        .collect();
    if dialogue.is_empty() {
        segments.to_vec()
    } else {
        dialogue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_narration_and_dialogue() {
        let segments = segment_text(
            "雨が窓を叩いていた。\n「今日は、少し遅いね」\n彼女は本を閉じた。",
            None,
        );
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].kind, SegmentType::Narration);
        assert_eq!(segments[1].kind, SegmentType::Dialogue);
        assert_eq!(segments[2].kind, SegmentType::Narration);
    }

    #[test]
    fn keeps_nested_quotes_inside_dialogue() {
        let segments = segment_text("「『吾輩は猫である』、読んだ？」", None);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].kind, SegmentType::Dialogue);
    }

    #[test]
    fn recognizes_character_name_prefix() {
        let segments = segment_text("七海：まだ帰らないの？", Some("七海"));
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].kind, SegmentType::Dialogue);
        assert_eq!(segments[0].text, "まだ帰らないの？");
    }

    #[test]
    fn ignores_unrelated_prefixes() {
        let segments = segment_text("状況：雨。教室は静かだ。", Some("七海"));
        assert_eq!(segments[0].kind, SegmentType::Narration);
    }

    #[test]
    fn treats_unclosed_quote_as_dialogue() {
        let segments = segment_text("「ちょっと待って\n彼女は振り返った。", None);
        assert_eq!(segments[0].kind, SegmentType::Dialogue);
        assert_eq!(segments[1].kind, SegmentType::Narration);
    }

    #[test]
    fn messenger_hides_narration_when_dialogue_exists() {
        let segments = segment_text("雨が降っていた。\n「傘、ある？」\n彼女は笑った。", None);
        let visible = visible_segments(&segments, "messenger");
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].text, "「傘、ある？」");
    }

    #[test]
    fn messenger_keeps_narration_when_there_is_no_dialogue() {
        let segments = segment_text("静かな夜だった。", None);
        assert_eq!(visible_segments(&segments, "messenger"), segments);
    }
}
