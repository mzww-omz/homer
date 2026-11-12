import { describe, expect, it } from "vitest";
import { segmentText, visibleSegments } from "./segments";

describe("segmentText", () => {
  it("separates narration and dialogue", () => {
    const segments = segmentText("雨が窓を叩いていた。\n「今日は、少し遅いね」\n彼女は本を閉じた。");
    expect(segments).toEqual([
      { type: "narration", text: "雨が窓を叩いていた。" },
      { type: "dialogue", text: "「今日は、少し遅いね」" },
      { type: "narration", text: "彼女は本を閉じた。" },
    ]);
  });

  it("keeps nested japanese quotes inside a line", () => {
    const segments = segmentText("「『吾輩は猫である』、読んだ？」");
    expect(segments).toHaveLength(1);
    expect(segments[0].type).toBe("dialogue");
  });

  it("handles multiple paragraphs of narration", () => {
    const segments = segmentText("窓の外は雨だった。\n\n教室には誰もいない。");
    expect(segments.filter((s) => s.type === "narration")).toHaveLength(2);
  });

  it("recognizes a character name prefix as dialogue", () => {
    const segments = segmentText("七海：まだ帰らないの？", "七海");
    expect(segments).toEqual([{ type: "dialogue", text: "まだ帰らないの？" }]);
  });

  it("does not treat other name-like prefixes as dialogue", () => {
    const segments = segmentText("状況：雨。教室は静かだ。", "七海");
    expect(segments[0].type).toBe("narration");
  });

  it("treats an unclosed quote line as dialogue", () => {
    const segments = segmentText("「ちょっと待って\n彼女は振り返った。");
    expect(segments[0].type).toBe("dialogue");
    expect(segments[1].type).toBe("narration");
  });

  it("returns an empty list for empty text", () => {
    expect(segmentText("   ")).toEqual([]);
  });
});

describe("visibleSegments", () => {
  const segments = segmentText("雨が降っていた。\n「傘、ある？」\n彼女は笑った。");

  it("shows everything in story mode", () => {
    expect(visibleSegments(segments, "story")).toHaveLength(3);
  });

  it("hides narration in messenger mode", () => {
    const visible = visibleSegments(segments, "messenger");
    expect(visible).toEqual([{ type: "dialogue", text: "「傘、ある？」" }]);
  });

  it("falls back to narration when there is no dialogue", () => {
    const narrationOnly = segmentText("静かな夜だった。");
    expect(visibleSegments(narrationOnly, "messenger")).toEqual(narrationOnly);
  });
});
