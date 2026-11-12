<script lang="ts">
  import type { Segment } from "../segments";
  import { segmentText, visibleSegments } from "../segments";

  let {
    content,
    mode = "story",
    characterName = "",
  }: {
    content: string;
    mode?: "story" | "messenger";
    characterName?: string;
  } = $props();

  const segments = $derived<Segment[]>(
    visibleSegments(segmentText(content, characterName), mode),
  );
</script>

<div class="body">
  {#each segments as segment}
    {#if segment.type === "dialogue"}
      <p class="dialogue">{segment.text}</p>
    {:else}
      <p class="narration">{segment.text}</p>
    {/if}
  {/each}
</div>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  p {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .dialogue {
    font-size: 15px;
    line-height: 1.82;
  }
  .narration {
    font-family: "Hiragino Mincho ProN", "Yu Mincho", "Noto Serif JP", serif;
    font-size: 14.5px;
    line-height: 1.9;
    color: #655d51;
    letter-spacing: 0.01em;
  }
</style>
