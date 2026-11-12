export type SegmentType = "narration" | "dialogue";

export interface Segment {
  type: SegmentType;
  text: string;
}

const QUOTE_PAIRS: Record<string, string> = {
  "「": "」",
  "『": "』",
  "“": "”",
};

function pushSegment(out: Segment[], type: SegmentType, text: string) {
  const trimmed = text.trim();
  if (!trimmed) return;
  const last = out[out.length - 1];
  if (last && last.type === type && type === "dialogue") {
    last.text += `\n${trimmed}`;
    return;
  }
  out.push({ type, text: trimmed });
}

/** Split model output into spoken lines and narration without interpreting markup. */
export function segmentText(raw: string, characterName?: string): Segment[] {
  const text = raw.replace(/\r\n?/g, "\n");
  const segments: Segment[] = [];
  let narration = "";

  const flushNarration = () => {
    if (!narration.trim()) {
      narration = "";
      return;
    }
    for (const block of narration.split(/\n{2,}/)) {
      const lines = block.split("\n");
      let pending = "";
      for (const line of lines) {
        const nameDialogue = matchNameDialogue(line, characterName);
        if (nameDialogue !== null) {
          if (pending.trim()) pushSegment(segments, "narration", pending);
          pending = "";
          pushSegment(segments, "dialogue", nameDialogue);
        } else if (isUnclosedQuote(line)) {
          if (pending.trim()) pushSegment(segments, "narration", pending);
          pending = "";
          pushSegment(segments, "dialogue", line.replace(/^[-—\s]*/, ""));
        } else {
          pending += (pending ? "\n" : "") + line;
        }
      }
      if (pending.trim()) pushSegment(segments, "narration", pending);
    }
    narration = "";
  };

  let i = 0;
  while (i < text.length) {
    const ch = text[i];
    const close = QUOTE_PAIRS[ch];
    if (close) {
      const end = text.indexOf(close, i + 1);
      if (end !== -1) {
        flushNarration();
        pushSegment(segments, "dialogue", text.slice(i, end + 1));
        i = end + 1;
        continue;
      }
    }
    narration += ch;
    i += 1;
  }
  flushNarration();
  return segments;
}

function matchNameDialogue(line: string, characterName?: string): string | null {
  if (!characterName) return null;
  const trimmed = line.trim().replace(/^[-—\s]+/, "");
  const escaped = characterName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = trimmed.match(new RegExp(`^${escaped}\\s*[：:]\\s*(.+)$`));
  return match ? match[1].trim() : null;
}

function isUnclosedQuote(line: string): boolean {
  const trimmed = line.trim();
  const first = trimmed[0];
  if (!first || !QUOTE_PAIRS[first]) return false;
  return !trimmed.slice(1).includes(QUOTE_PAIRS[first]);
}

/** Messenger mode hides narration unless there is no spoken line at all. */
export function visibleSegments(segments: Segment[], mode: "story" | "messenger"): Segment[] {
  if (mode === "story") return segments;
  const dialogue = segments.filter((s) => s.type === "dialogue");
  return dialogue.length > 0 ? dialogue : segments;
}

export function formatTime(ts: number): string {
  const date = new Date(ts);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return date.toLocaleTimeString("ja-JP", { hour: "2-digit", minute: "2-digit" });
  }
  return date.toLocaleDateString("ja-JP", { month: "numeric", day: "numeric" });
}
