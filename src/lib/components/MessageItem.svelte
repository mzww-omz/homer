<script lang="ts">
  import type { Character, Message } from "../types";
  import { formatTime } from "../segments";
  import Avatar from "./Avatar.svelte";
  import MessageBody from "./MessageBody.svelte";

  let {
    message,
    character = null,
    mode = "story",
    showMetrics = false,
  }: {
    message: Message;
    character?: Character | null;
    mode?: "story" | "messenger";
    showMetrics?: boolean;
  } = $props();

  const metrics = $derived(message.meta?.metrics);
  const name = $derived(character?.name ?? "語り手");
</script>

{#if message.role === "user"}
  <div class="row user">
    <div class="user-wrap">
      <div class="bubble">{message.content}</div>
      <span class="time">{formatTime(message.created_at)}</span>
    </div>
  </div>
{:else}
  <div class="row assistant">
    <div class="meta">
      {#if character}
        <Avatar name={character.name} accent={character.accent} size={22} />
      {:else}
        <span class="narrator-avatar">語</span>
      {/if}
      <span class="name">{name}</span>
      <span class="time">{formatTime(message.created_at)}</span>
      {#if message.meta?.stopped}
        <span class="tag">中断</span>
      {/if}
      {#if message.meta?.incomplete}
        <span class="tag">未完了</span>
      {/if}
    </div>
    <div class="content">
      <MessageBody content={message.content} {mode} characterName={name} />
      {#if showMetrics && metrics}
        <div class="metrics">
          <span>TTFT {(metrics.ttft_ms / 1000).toFixed(2)}s</span>
          <span>{metrics.tokens_per_sec.toFixed(1)} tok/s</span>
          <span>{(metrics.duration_ms / 1000).toFixed(1)}s</span>
          <span>{metrics.estimated_tokens} tok</span>
          {#if metrics.trimmed_messages > 0}
            <span>履歴 {metrics.trimmed_messages} 件省略</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .row {
    display: flex;
    width: 100%;
  }
  .row.user {
    justify-content: flex-end;
    padding: 6px 0;
  }
  .assistant {
    flex-direction: column;
    padding: 10px 0;
  }
  .user-wrap {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 3px;
    max-width: 78%;
  }
  .bubble {
    background: #eae3d6;
    border: 1px solid #e2d9ca;
    padding: 8px 13px;
    border-radius: 14px 14px 3px 14px;
    font-size: 14.5px;
    line-height: 1.75;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 6px;
    min-height: 22px;
  }
  .narrator-avatar {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--faint);
    font-size: 10px;
    font-family: "Hiragino Mincho ProN", "Yu Mincho", serif;
  }
  .name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: 0.01em;
  }
  .time {
    font-size: 10.5px;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .user .time {
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .user:hover .time {
    opacity: 1;
  }
  .tag {
    font-size: 9.5px;
    padding: 1px 5px;
    border: 1px solid var(--line);
    border-radius: 4px;
    color: var(--faint);
    letter-spacing: 0.04em;
  }
  .content {
    padding-left: 29px;
  }
  .metrics {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 8px;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 10px;
    color: var(--faint);
    letter-spacing: -0.01em;
  }
</style>
