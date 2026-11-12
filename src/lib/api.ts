import { invoke } from "@tauri-apps/api/core";
import type {
  Character,
  CharacterInput,
  ConnectionInfo,
  Conversation,
  ConversationInput,
  ConversationPatch,
  Message,
  Settings,
  SettingsPatch,
} from "./types";

export const api = {
  listCharacters: () => invoke<Character[]>("list_characters"),
  createCharacter: (input: CharacterInput) =>
    invoke<Character>("create_character", { input }),
  updateCharacter: (id: string, input: CharacterInput) =>
    invoke<Character>("update_character", { id, input }),
  deleteCharacter: (id: string) => invoke<void>("delete_character", { id }),

  listConversations: () => invoke<Conversation[]>("list_conversations"),
  createConversation: (input: ConversationInput) =>
    invoke<Conversation>("create_conversation", { input }),
  updateConversation: (id: string, patch: ConversationPatch) =>
    invoke<Conversation>("update_conversation", { id, patch }),
  deleteConversation: (id: string) => invoke<void>("delete_conversation", { id }),

  listMessages: (conversationId: string) =>
    invoke<Message[]>("list_messages", { conversationId }),
  sendMessage: (conversationId: string, content: string) =>
    invoke<Message>("send_message", { conversationId, content }),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (patch: SettingsPatch) =>
    invoke<Settings>("save_settings", { patch }),
  setApiKey: (key: string | null) => invoke<boolean>("set_api_key", { key }),
  hasApiKey: () => invoke<boolean>("has_api_key"),

  listModels: (baseUrl: string, apiKey?: string) =>
    invoke<string[]>("list_models", { baseUrl, apiKey: apiKey ?? null }),
  testConnection: (baseUrl: string, apiKey?: string) =>
    invoke<ConnectionInfo>("test_connection", {
      baseUrl,
      apiKey: apiKey ?? null,
    }),

  streamReply: (conversationId: string, requestId: string) =>
    invoke<void>("stream_reply", { conversationId, requestId }),
  cancelGeneration: (requestId: string) =>
    invoke<boolean>("cancel_generation", { requestId }),
};
