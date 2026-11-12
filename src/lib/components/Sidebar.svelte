<script lang="ts">
  import { store } from "../store.svelte";
  import type { Character } from "../types";
  import { formatTime } from "../segments";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";

  let {
    onnewconversation,
    oneditcharacter,
    onsettings,
  }: {
    onnewconversation: () => void;
    oneditcharacter: (character: Character | null) => void;
    onsettings: () => void;
  } = $props();

  let tab = $state<"conversations" | "characters">("conversations");

  function subtitle(id: string) {
    const conversation = store.conversations.find((c) => c.id === id);
    if (!conversation) return "";
    const character = conversation.character_id
      ? store.characters.find((c) => c.id === conversation.character_id)
      : null;
    const scene = conversation.scene || conversation.scenario;
    if (scene) return scene.replace(/\s+/g, " ").slice(0, 48);
    return character?.description.replace(/\s+/g, " ").slice(0, 48) || "物語";
  }
</script>

<aside class="sidebar">
  <div class="top">
    <div class="brand">Homer</div>
    <button
      class="icon-btn"
      aria-label={tab === "conversations" ? "新しい会話" : "新しいキャラクター"}
      title={tab === "conversations" ? "新しい会話（⌘N）" : "新しいキャラクター"}
      onclick={() => (tab === "conversations" ? onnewconversation() : oneditcharacter(null))}
    >
      <Icon name="plus" size={16} />
    </button>
  </div>

  <div class="tabs" role="tablist">
    <button
      role="tab"
      aria-selected={tab === "conversations"}
      class:active={tab === "conversations"}
      onclick={() => (tab = "conversations")}>会話</button
    >
    <button
      role="tab"
      aria-selected={tab === "characters"}
      class:active={tab === "characters"}
      onclick={() => (tab = "characters")}>キャラクター</button
    >
  </div>

  <div class="list">
    {#if tab === "conversations"}
      {#if store.conversations.length === 0}
        <div class="empty-list">
          <p>まだ物語がありません。</p>
          <button class="text-link" onclick={onnewconversation}>新しい会話を始める</button>
        </div>
      {:else}
        {#each store.conversations as conversation (conversation.id)}
          {@const character = conversation.character_id
            ? store.characters.find((c) => c.id === conversation.character_id)
            : null}
          <button
            class="row"
            class:active={store.activeId === conversation.id}
            onclick={() => store.selectConversation(conversation.id)}
            title={conversation.title}
          >
            {#if character}
              <Avatar name={character.name} accent={character.accent} size={30} />
            {:else}
              <span class="narrator">語</span>
            {/if}
            <span class="row-text">
              <span class="row-top">
                <span class="row-title">{conversation.title}</span>
                <span class="row-time">{formatTime(conversation.updated_at)}</span>
              </span>
              <span class="row-sub">{subtitle(conversation.id)}</span>
            </span>
          </button>
        {/each}
      {/if}
    {:else}
      {#if store.characters.length === 0}
        <div class="empty-list">
          <p>キャラクターはまだいません。</p>
          <button class="text-link" onclick={() => oneditcharacter(null)}>
            キャラクターを作る
          </button>
        </div>
      {:else}
        {#each store.characters as character (character.id)}
          <button class="row" onclick={() => oneditcharacter(character)}>
            <Avatar name={character.name} accent={character.accent} size={30} />
            <span class="row-text">
              <span class="row-top">
                <span class="row-title">{character.name}</span>
              </span>
              <span class="row-sub">
                {character.description.replace(/\s+/g, " ").slice(0, 52) || "設定なし"}
              </span>
            </span>
          </button>
        {/each}
      {/if}
    {/if}
  </div>

  <div class="footer">
    <button class="settings-btn" onclick={onsettings}>
      <Icon name="settings" size={15} />
      <span>設定</span>
    </button>
    <span class="model-hint" class:hidden={!store.settings.model} title={store.settings.model}>
      {store.settings.model || "モデル未設定"}
    </span>
  </div>
</aside>

<style>
  .sidebar {
    width: 276px;
    min-width: 276px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
    user-select: none;
  }
  .top {
    height: 52px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 0 18px;
    flex: none;
  }
  .brand {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: #4b443b;
  }
  .tabs {
    display: flex;
    gap: 14px;
    padding: 0 18px;
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .tabs button {
    padding: 0 0 8px;
    font-size: 11.5px;
    color: var(--faint);
    border-bottom: 1.5px solid transparent;
    margin-bottom: -1px;
    transition: color 120ms ease;
  }
  .tabs button:hover {
    color: var(--muted);
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--text);
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 7px 9px;
    border-radius: 7px;
    text-align: left;
    min-height: 49px;
    transition: background 100ms ease;
  }
  .row:hover {
    background: rgba(60, 52, 43, 0.045);
  }
  .row.active {
    background: #e3dccf;
  }
  .narrator {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--line-strong);
    color: var(--faint);
    font-family: "Hiragino Mincho ProN", "Yu Mincho", serif;
    font-size: 12px;
    flex: none;
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .row-top {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .row-title {
    font-size: 12.5px;
    font-weight: 550;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }
  .row-time {
    font-size: 9.5px;
    color: var(--faint);
    flex: none;
    font-variant-numeric: tabular-nums;
  }
  .row-sub {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
  }
  .empty-list {
    padding: 28px 12px;
    text-align: center;
    color: var(--faint);
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
  }
  .empty-list p {
    margin: 0;
  }
  .text-link {
    font-size: 12px;
    color: var(--muted);
    text-decoration: underline;
    text-underline-offset: 3px;
    text-decoration-color: var(--line-strong);
  }
  .text-link:hover {
    color: var(--text);
  }
  .footer {
    border-top: 1px solid var(--line);
    height: 48px;
    display: flex;
    align-items: center;
    padding: 0 8px;
    flex: none;
    gap: 6px;
  }
  .settings-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 9px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  .settings-btn:hover {
    background: rgba(60, 52, 43, 0.05);
    color: var(--text);
  }
  .model-hint {
    margin-left: auto;
    margin-right: 6px;
    max-width: 132px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 9.5px;
    color: var(--faint);
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
  }
  .model-hint.hidden {
    color: var(--faint);
    opacity: 0.7;
  }

  @media (max-width: 950px) {
    .sidebar {
      width: 242px;
      min-width: 242px;
    }
    .row-time {
      display: none;
    }
  }
</style>
