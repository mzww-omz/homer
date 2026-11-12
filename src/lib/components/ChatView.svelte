<script lang="ts">
  import { tick } from "svelte";
  import { store } from "../store.svelte";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";
  import MessageItem from "./MessageItem.svelte";
  import MessageBody from "./MessageBody.svelte";

  let {
    onopenpanel,
    oneditcharacter,
    onnewconversation,
  }: {
    onopenpanel: () => void;
    oneditcharacter: () => void;
    onnewconversation: () => void;
  } = $props();

  let draft = $state("");
  let inputEl: HTMLTextAreaElement | undefined = $state();
  let scroller: HTMLDivElement | undefined = $state();
  let pinned = true;
  let lastConvId: string | null = null;

  const conversation = $derived(store.activeConversation);
  const character = $derived(store.activeCharacter);
  const streamingHere = $derived(
    store.streaming && store.streamConversationId === store.activeId,
  );

  function scrollToBottom() {
    if (!scroller) return;
    scroller.scrollTop = scroller.scrollHeight;
  }

  function onscroll() {
    if (!scroller) return;
    pinned =
      scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 90;
  }

  $effect(() => {
    const id = store.activeId;
    const count = store.messages.length;
    const streamed = store.streamText;
    void count;
    void streamed;
    if (!scroller) return;
    if (id !== lastConvId) {
      lastConvId = id;
      pinned = true;
      draft = "";
      void tick().then(() => {
        scrollToBottom();
        resize();
        inputEl?.focus();
      });
      return;
    }
    if (pinned) void tick().then(scrollToBottom);
  });

  function resize() {
    if (!inputEl) return;
    inputEl.style.height = "auto";
    inputEl.style.height = `${Math.min(inputEl.scrollHeight, 168)}px`;
  }

  async function send() {
    const text = draft;
    if (!text.trim() || store.streaming || store.sending) return;
    draft = "";
    resize();
    const ok = await store.send(text);
    if (!ok && !draft) {
      draft = text;
      void tick().then(resize);
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (
      event.key === "Enter" &&
      !event.shiftKey &&
      !event.isComposing &&
      event.keyCode !== 229
    ) {
      event.preventDefault();
      void send();
    }
  }

  async function setMode(mode: "story" | "messenger") {
    if (!conversation || conversation.mode === mode) return;
    try {
      await store.updateConversation(conversation.id, { mode });
    } catch (e) {
      store.notify(String(e));
    }
  }

  function deleteConversation() {
    if (!conversation) return;
    if (
      !confirm(
        `「${conversation.title}」を削除しますか？\nこの会話のメッセージも削除されます。`,
      )
    )
      return;
    void store.deleteConversation(conversation.id).catch((e) => store.notify(String(e)));
  }
</script>

<section class="chat">
  {#if conversation}
    <header class="header">
      <button class="identity" onclick={onopenpanel} title="場面とキャラクターの設定">
        {#if character}
          <Avatar name={character.name} accent={character.accent} size={30} />
        {:else}
          <span class="narrator-avatar">語</span>
        {/if}
        <span class="identity-text">
          <span class="title">{conversation.title}</span>
          <span class="subtitle">
            {conversation.scenario ||
              character?.description ||
              conversation.scene ||
              "自由な物語"}
          </span>
        </span>
      </button>

      <div class="header-actions">
        <div class="segmented" role="group" aria-label="表示形式">
          <button
            class:active={conversation.mode === "story"}
            onclick={() => setMode("story")}
            title="地の文を含む物語形式"
          >
            <Icon name="book" size={12} />物語
          </button>
          <button
            class:active={conversation.mode === "messenger"}
            onclick={() => setMode("messenger")}
            title="会話中心のメッセンジャー形式"
          >
            <Icon name="message" size={12} />メッセンジャー
          </button>
        </div>
        <button class="icon-btn" onclick={onopenpanel} title="この物語の設定">
          <Icon name="panel" size={15} />
        </button>
      </div>
    </header>

    <div class="scroller" bind:this={scroller} onscroll={onscroll}>
      <div class="thread">
        {#if store.loadingMessages}
          <div class="thread-loading">読み込み中…</div>
        {:else if store.messages.length === 0 && !streamingHere}
          <div class="intro">
            {#if character}
              <Avatar name={character.name} accent={character.accent} size={52} />
            {:else}
              <span class="narrator-avatar large">語</span>
            {/if}
            <h1>{conversation.title}</h1>
            {#if conversation.scenario}
              <p class="scenario">{conversation.scenario}</p>
            {/if}
            {#if character?.description}
              <p class="description">{character.description}</p>
            {/if}
            <p class="first-hint">最初の言葉を書いてください。</p>
            <button class="quiet-link" onclick={oneditcharacter}>
              {character ? "キャラクターを編集" : "キャラクターを作る"}
            </button>
          </div>
        {:else}
          {#each store.messages as message (message.id)}
            <MessageItem
              {message}
              character={store.activeCharacter}
              mode={conversation.mode}
              showMetrics={store.settings.show_metrics}
            />
          {/each}
        {/if}

        {#if streamingHere}
          <div class="streaming">
            <div class="stream-meta">
              {#if store.activeCharacter}
                <Avatar
                  name={store.activeCharacter.name}
                  accent={store.activeCharacter.accent}
                  size={22}
                />
              {:else}
                <span class="narrator-avatar">語</span>
              {/if}
              <span class="stream-name">{store.activeCharacter?.name ?? "語り手"}</span>
              {#if !store.streamText}
                <span class="thinking" aria-label="生成中"
                  ><span></span><span></span><span></span
                ></span>
              {/if}
            </div>
            <div class="stream-content">
              {#if store.streamText}
                <MessageBody
                  content={store.streamText}
                  mode={conversation.mode}
                  characterName={store.activeCharacter?.name ?? ""}
                />
              {/if}
              <span class="caret"></span>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="composer-area">
      {#if store.error}
        <div class="error-banner" role="alert">
          <Icon name="alert" size={15} />
          <div class="error-text">
            <p>{store.error.message}</p>
            <span class="error-kind">
              {store.error.kind === "network"
                ? "接続"
                : store.error.kind === "http"
                  ? "サーバー応答"
                  : store.error.kind === "persistence"
                    ? "保存"
                    : "生成"}
            </span>
          </div>
          <div class="error-actions">
            {#if store.messages.some((m) => m.role === "user")}
              <button
                class="btn"
                disabled={store.streaming}
                onclick={() => store.retry()}>再試行</button
              >
            {/if}
            <button class="icon-btn" aria-label="閉じる" onclick={() => store.dismissError()}>
              <Icon name="close" size={14} />
            </button>
          </div>
        </div>
      {/if}

      {#if streamingHere}
        <div class="generation-bar">
          <span class="generation-label">
            <span class="pulse"></span>
            応答を生成中
          </span>
          <button class="stop-btn" onclick={() => store.cancel()}>
            <Icon name="stop" size={9} />
            停止
          </button>
        </div>
      {/if}

      <div class="composer">
        <textarea
          bind:this={inputEl}
          bind:value={draft}
          oninput={resize}
          onkeydown={onkeydown}
          rows="1"
          placeholder={character
            ? `${character.name}に話しかける…`
            : "言葉を書く…"}
          aria-label="メッセージ"
          spellcheck="false"
        ></textarea>
        <div class="composer-tools">
          <span class="send-hint">Enterで送信 · Shift+Enterで改行</span>
          <button
            class="send-btn"
            onclick={send}
            disabled={!draft.trim() || store.streaming || store.sending}
            aria-label="送信"
            title="送信"
          >
            <Icon name="send" size={15} strokeWidth={2} />
          </button>
        </div>
      </div>
      <div class="under-composer">
        <button class="quiet-link small" onclick={deleteConversation}>この会話を削除</button>
      </div>
    </div>
  {:else}
    <div class="no-chat">
      <div class="no-chat-inner">
        <span class="narrator-avatar large quiet">語</span>
        <p class="no-chat-title">物語は、まだ始まっていません。</p>
        <p class="no-chat-text">
          キャラクターと場面を決めて、会話を始めましょう。
        </p>
        <div class="no-chat-actions">
          <button class="btn btn-primary" onclick={onnewconversation}>新しい会話</button>
          <button class="btn" onclick={oneditcharacter}>キャラクターを作る</button>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .chat {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    height: 100%;
    background: var(--bg);
    position: relative;
  }
  .header {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 16px 0 20px;
    border-bottom: 1px solid var(--line);
    background: var(--bg);
  }
  .identity {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    text-align: left;
    padding: 4px 6px 4px 0;
    border-radius: 7px;
  }
  .identity:hover .title {
    color: #000;
  }
  .narrator-avatar {
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
  .narrator-avatar.large {
    width: 52px;
    height: 52px;
    font-size: 18px;
  }
  .identity-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }
  .title {
    font-size: 13.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtitle {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 360px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
  .scroller {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .thread {
    max-width: 768px;
    margin: 0 auto;
    padding: 26px 32px 32px;
    min-height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }
  .thread-loading {
    color: var(--faint);
    font-size: 12px;
    text-align: center;
    margin: auto;
  }
  .intro {
    margin: auto;
    max-width: 470px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 24px 0 40px;
  }
  .intro h1 {
    font-size: 18px;
    font-weight: 600;
    margin: 6px 0 0;
    letter-spacing: 0.01em;
  }
  .scenario {
    margin: 2px 0 0;
    font-size: 13px;
    line-height: 1.75;
    color: #5f574c;
    font-family: "Hiragino Mincho ProN", "Yu Mincho", serif;
  }
  .description {
    margin: 0;
    font-size: 12px;
    line-height: 1.7;
    color: var(--muted);
  }
  .first-hint {
    margin: 14px 0 0;
    font-size: 12px;
    color: var(--faint);
  }
  .quiet-link {
    font-size: 11.5px;
    color: var(--muted);
    text-decoration: underline;
    text-underline-offset: 3px;
    text-decoration-color: var(--line-strong);
  }
  .quiet-link:hover {
    color: var(--text);
  }
  .quiet-link.small {
    font-size: 10.5px;
    color: var(--faint);
    text-decoration: none;
  }
  .quiet-link.small:hover {
    color: var(--muted);
  }
  .streaming {
    padding: 10px 0;
    display: flex;
    flex-direction: column;
  }
  .stream-meta {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 6px;
    min-height: 22px;
  }
  .stream-name {
    font-size: 12px;
    font-weight: 600;
  }
  .stream-content {
    padding-left: 29px;
  }
  .caret {
    display: inline-block;
    width: 1px;
    height: 15px;
    background: var(--text);
    vertical-align: -3px;
    margin-left: 1px;
    animation: blink 1s steps(2, start) infinite;
  }
  .thinking {
    display: inline-flex;
    gap: 3px;
    align-items: center;
    margin-left: 2px;
  }
  .thinking span {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--faint);
    animation: bounce 1.2s infinite ease-in-out;
  }
  .thinking span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .thinking span:nth-child(3) {
    animation-delay: 0.3s;
  }
  .composer-area {
    flex: none;
    padding: 0 16px 8px;
    background: var(--bg);
  }
  .error-banner,
  .generation-bar,
  .composer,
  .under-composer {
    max-width: 768px;
    margin-left: auto;
    margin-right: auto;
  }
  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 10px 12px;
    margin-bottom: 8px;
    border: 1px solid #e2c8bf;
    background: #f8ece7;
    border-radius: 8px;
    color: #6d3f33;
  }
  .error-banner :global(svg) {
    margin-top: 2px;
    flex: none;
  }
  .error-text {
    flex: 1;
    min-width: 0;
  }
  .error-text p {
    margin: 0;
    font-size: 12px;
    line-height: 1.65;
    overflow-wrap: anywhere;
  }
  .error-kind {
    font-size: 9.5px;
    letter-spacing: 0.06em;
    color: #a3796c;
  }
  .error-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .error-actions .btn {
    height: 26px;
    font-size: 11.5px;
    background: #fffaf7;
  }
  .generation-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px 6px;
  }
  .generation-label {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 11px;
    color: var(--muted);
  }
  .pulse {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.4s infinite ease-in-out;
  }
  .stop-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--muted);
    border: 1px solid var(--line);
    background: var(--surface);
    border-radius: 5px;
    height: 23px;
    padding: 0 8px;
  }
  .stop-btn:hover {
    color: var(--text);
    border-color: var(--line-strong);
  }
  .composer {
    border: 1px solid var(--line-strong);
    background: var(--surface);
    border-radius: 10px;
    padding: 10px 10px 8px 13px;
    box-shadow: 0 1px 2px rgba(52, 44, 35, 0.03);
    transition: border-color 120ms ease;
  }
  .composer:focus-within {
    border-color: #b8ad9c;
  }
  .composer textarea {
    width: 100%;
    border: 0;
    background: transparent;
    padding: 0;
    margin: 0;
    resize: none;
    font-size: 14.5px;
    line-height: 1.72;
    min-height: 24px;
    max-height: 168px;
    outline: none;
  }
  .composer textarea::placeholder {
    color: #b3aa9c;
  }
  .composer-tools {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 5px;
  }
  .send-hint {
    font-size: 10px;
    color: #b0a79a;
    padding-left: 1px;
  }
  .send-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #38322b;
    color: #fbf8f1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    transition:
      background 120ms ease,
      opacity 120ms ease;
  }
  .send-btn:hover:not(:disabled) {
    background: #1f1b17;
  }
  .send-btn:disabled {
    background: #d9d1c4;
    color: #f6f1e8;
    opacity: 1;
  }
  .under-composer {
    display: flex;
    justify-content: flex-end;
    padding: 5px 2px 0;
  }
  .no-chat {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px;
  }
  .no-chat-inner {
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    max-width: 380px;
  }
  .narrator-avatar.quiet {
    opacity: 0.55;
    margin-bottom: 6px;
  }
  .no-chat-title {
    font-size: 16px;
    font-weight: 550;
    margin: 0;
  }
  .no-chat-text {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.7;
  }
  .no-chat-actions {
    display: flex;
    gap: 8px;
    margin-top: 14px;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  @keyframes bounce {
    0%,
    70%,
    100% {
      transform: translateY(0);
      opacity: 0.55;
    }
    35% {
      transform: translateY(-2px);
      opacity: 1;
    }
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 1;
    }
  }

  @media (max-width: 950px) {
    .thread {
      padding: 20px 20px 24px;
    }
    .subtitle {
      max-width: 220px;
    }
    .composer-area {
      padding: 0 12px 6px;
    }
  }
</style>
