export interface Character {
  id: string;
  name: string;
  description: string;
  personality: string;
  speech_style: string;
  first_person: string;
  relationship: string;
  role: string;
  example_dialogue: string;
  extra: string;
  accent: string;
  created_at: number;
  updated_at: number;
}

export type CharacterInput = Omit<Character, "id" | "created_at" | "updated_at">;

export interface Conversation {
  id: string;
  character_id: string | null;
  title: string;
  scenario: string;
  scene: string;
  mode: "story" | "messenger";
  notes: string;
  created_at: number;
  updated_at: number;
}

export interface ConversationInput {
  character_id: string | null;
  title: string;
  scenario: string;
  scene: string;
  mode: "story" | "messenger";
  notes: string;
}

export interface ConversationPatch {
  title?: string;
  scenario?: string;
  scene?: string;
  mode?: "story" | "messenger";
  notes?: string;
}

export interface GenMetrics {
  ttft_ms: number;
  duration_ms: number;
  chars: number;
  estimated_tokens: number;
  tokens_per_sec: number;
  context_tokens: number;
  trimmed_messages: number;
}

export interface MessageMeta {
  metrics?: GenMetrics;
  stopped?: boolean;
  incomplete?: boolean;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: "user" | "assistant";
  content: string;
  created_at: number;
  meta: MessageMeta | null;
}

export interface Settings {
  base_url: string;
  model: string;
  temperature: number;
  top_p: number;
  max_tokens: number;
  context_budget: number;
  timeout_secs: number;
  show_metrics: boolean;
}

export type SettingsPatch = Partial<Settings>;

export interface ConnectionInfo {
  ok: boolean;
  models: string[];
  message: string;
}

export interface DeltaPayload {
  request_id: string;
  delta: string;
}

export interface DonePayload {
  request_id: string;
  message: Message;
  metrics: GenMetrics;
  stopped: boolean;
}

export interface ErrorPayload {
  request_id: string;
  message: string;
  kind: string;
  partial: string;
}
