import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "./api";
import type {
  Character,
  CharacterInput,
  Conversation,
  ConversationInput,
  ConversationPatch,
  DeltaPayload,
  DonePayload,
  ErrorPayload,
  Message,
  Settings,
  SettingsPatch,
} from "./types";

const DEFAULT_SETTINGS: Settings = {
  base_url: "http://localhost:1234/v1",
  model: "",
  temperature: 0.85,
  top_p: 0.95,
  max_tokens: 400,
  context_budget: 6000,
  timeout_secs: 120,
  show_metrics: false,
};

export interface AppError {
  message: string;
  kind: string;
}

class AppStore {
  characters = $state<Character[]>([]);
  conversations = $state<Conversation[]>([]);
  messages = $state<Message[]>([]);
  settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  activeId = $state<string | null>(null);

  loading = $state(true);
  loadingMessages = $state(false);
  fatalError = $state<string | null>(null);
  streaming = $state(false);
  streamText = $state("");
  streamConversationId = $state<string | null>(null);
  sending = $state(false);
  error = $state<AppError | null>(null);
  hasApiKey = $state(false);
  toast = $state<string | null>(null);

  activeConversation = $derived(
    this.conversations.find((c) => c.id === this.activeId) ?? null,
  );
  activeCharacter = $derived(
    this.activeConversation?.character_id
      ? (this.characters.find((c) => c.id === this.activeConversation?.character_id) ??
        null)
      : null,
  );

  private requestId: string | null = null;
  private sawError = false;
  private pending = "";
  private raf: number | null = null;
  private unlisteners: UnlistenFn[] = [];
  private initialized = false;
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  async init() {
    if (this.initialized) return;
    this.initialized = true;
    try {
      await this.bindEvents();
      const [settings, characters, conversations, hasKey] = await Promise.all([
        api.getSettings(),
        api.listCharacters(),
        api.listConversations(),
        api.hasApiKey(),
      ]);
      this.settings = settings;
      this.characters = characters;
      this.conversations = conversations;
      this.hasApiKey = hasKey;
      if (conversations.length > 0) {
        await this.selectConversation(conversations[0].id);
      }
    } catch (e) {
      this.fatalError = String(e);
    } finally {
      this.loading = false;
    }
  }

  dispose() {
    this.unlisteners.forEach((un) => un());
    this.unlisteners = [];
  }

  private async bindEvents() {
    const delta = await listen<DeltaPayload>("llm:delta", (event) => {
      if (event.payload.request_id !== this.requestId) return;
      this.queueDelta(event.payload.delta);
    });
    const done = await listen<DonePayload>("llm:done", (event) => {
      this.handleDone(event.payload);
    });
    const error = await listen<ErrorPayload>("llm:error", (event) => {
      this.handleError(event.payload);
    });
    this.unlisteners.push(delta, done, error);
  }

  private queueDelta(delta: string) {
    this.pending += delta;
    if (this.raf !== null) return;
    this.raf = requestAnimationFrame(() => {
      this.raf = null;
      if (this.pending) {
        this.streamText += this.pending;
        this.pending = "";
      }
    });
  }

  private flushPending() {
    if (this.raf !== null) {
      cancelAnimationFrame(this.raf);
      this.raf = null;
    }
    if (this.pending) {
      this.streamText += this.pending;
      this.pending = "";
    }
  }

  private handleDone(payload: DonePayload) {
    if (payload.request_id !== this.requestId) {
      void this.reloadConversations();
      return;
    }
    this.flushPending();
    if (payload.message.conversation_id === this.activeId) {
      this.messages.push(payload.message);
    }
    this.streaming = false;
    this.streamText = "";
    this.requestId = null;
    this.streamConversationId = null;
    void this.reloadConversations();
  }

  private handleError(payload: ErrorPayload) {
    if (payload.request_id !== this.requestId) return;
    this.sawError = true;
    this.flushPending();
    const convId = this.streamConversationId;
    this.streaming = false;
    this.streamText = "";
    this.requestId = null;
    this.streamConversationId = null;
    this.error = { message: payload.message, kind: payload.kind };
    if (payload.partial && convId && convId === this.activeId) {
      void this.reloadMessages(convId);
    }
    void this.reloadConversations();
  }

  async reloadConversations() {
    try {
      this.conversations = await api.listConversations();
    } catch {
      /* keep the current list; generation results are already visible */
    }
  }

  async selectConversation(id: string | null) {
    if (id === this.activeId) return;
    if (this.streaming && this.requestId) {
      void api.cancelGeneration(this.requestId);
      this.streaming = false;
      this.streamText = "";
      this.pending = "";
      this.requestId = null;
      this.streamConversationId = null;
    }
    this.activeId = id;
    this.error = null;
    if (!id) {
      this.messages = [];
      return;
    }
    await this.reloadMessages(id);
  }

  async reloadMessages(conversationId: string) {
    this.loadingMessages = true;
    try {
      const messages = await api.listMessages(conversationId);
      if (this.activeId === conversationId) this.messages = messages;
    } catch (e) {
      this.toast = String(e);
    } finally {
      this.loadingMessages = false;
    }
  }

  async createConversation(input: ConversationInput) {
    const conversation = await api.createConversation(input);
    await this.reloadConversations();
    await this.selectConversation(conversation.id);
    return conversation;
  }

  async updateConversation(id: string, patch: ConversationPatch) {
    const updated = await api.updateConversation(id, patch);
    this.conversations = this.conversations.map((c) =>
      c.id === id ? updated : c,
    );
    return updated;
  }

  async deleteConversation(id: string) {
    await api.deleteConversation(id);
    const remaining = this.conversations.filter((c) => c.id !== id);
    this.conversations = remaining;
    if (this.activeId === id) {
      this.activeId = null;
      this.messages = [];
      if (remaining.length > 0) await this.selectConversation(remaining[0].id);
    }
  }

  async saveCharacter(id: string | null, input: CharacterInput) {
    const character = id
      ? await api.updateCharacter(id, input)
      : await api.createCharacter(input);
    const exists = this.characters.some((c) => c.id === character.id);
    this.characters = exists
      ? this.characters.map((c) => (c.id === character.id ? character : c))
      : [character, ...this.characters];
    return character;
  }

  async deleteCharacter(id: string) {
    await api.deleteCharacter(id);
    this.characters = this.characters.filter((c) => c.id !== id);
    await this.reloadConversations();
  }

  async saveSettings(patch: SettingsPatch) {
    const settings = await api.saveSettings(patch);
    this.settings = settings;
    return settings;
  }

  async setApiKey(key: string | null) {
    this.hasApiKey = await api.setApiKey(key);
    return this.hasApiKey;
  }

  async send(rawText: string) {
    const text = rawText.trim();
    const conversationId = this.activeId;
    if (!text || !conversationId || this.streaming || this.sending) return;
    this.sending = true;
    this.error = null;
    try {
      const message = await api.sendMessage(conversationId, text);
      if (this.activeId === conversationId) {
        this.messages.push(message);
      }
      await this.reloadConversations();
    } catch (e) {
      this.error = { message: String(e), kind: "persistence" };
      this.sending = false;
      return;
    }
    this.sending = false;
    await this.runGeneration(conversationId);
  }

  async retry() {
    const conversationId = this.activeId;
    if (!conversationId || this.streaming || this.sending) return;
    this.error = null;
    await this.runGeneration(conversationId);
  }

  private async runGeneration(conversationId: string) {
    const requestId = crypto.randomUUID();
    this.requestId = requestId;
    this.streamConversationId = conversationId;
    this.streaming = true;
    this.streamText = "";
    this.pending = "";
    this.sawError = false;
    try {
      await api.streamReply(conversationId, requestId);
    } catch (e) {
      if (this.requestId === requestId && !this.sawError) {
        this.error = { message: String(e), kind: "unknown" };
      }
    } finally {
      if (this.requestId === requestId) {
        this.streaming = false;
        this.streamText = "";
        this.requestId = null;
        this.streamConversationId = null;
      }
    }
  }

  async cancel() {
    if (!this.requestId) return;
    await api.cancelGeneration(this.requestId);
  }

  dismissError() {
    this.error = null;
  }

  notify(message: string) {
    this.toast = message;
    if (this.toastTimer) clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => {
      this.toast = null;
    }, 2600);
  }
}

export const store = new AppStore();
