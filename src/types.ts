export type SnippetType =
  | "PROMPT"
  | "PAYLOAD"
  | "EXP"
  | "COMMAND"
  | "CODE"
  | "TEXT"
  | "TEMPLATE"
  | "HTTP";

export const SNIPPET_TYPES: SnippetType[] = [
  "PROMPT",
  "PAYLOAD",
  "EXP",
  "COMMAND",
  "CODE",
  "TEXT",
  "TEMPLATE",
  "HTTP",
];

export interface Snippet {
  id: number;
  title: string;
  content: string;
  type: SnippetType;
  category: string;
  language: string;
  tags: string[];
  aliases: string[];
  favorite: boolean;
  usageCount: number;
  lastUsedAt: number | null;
  createdAt: number;
  updatedAt: number;
  kind: "TEXT" | "COMMAND_GROUP";
  commands: string[];
  dockPinned?:boolean;stableId?:string;sourcePackId?:string;sourcePackVersion?:string;
}

export interface SnippetInput {
  title: string;
  content: string;
  type: SnippetType;
  category: string;
  language: string;
  tags: string[];
  aliases: string[];
  kind: "TEXT" | "COMMAND_GROUP";
  commands: string[];
}

export interface CategoryInfo {
  name: string;
  count: number;
}

export interface CopyResult {
  text: string;
  snippet: Snippet;
}

export interface ScratchItem { id:number; content:string; createdAt:number; updatedAt:number }
export type ViewName = "all" | "favorites" | "recent" | "most_used" | "dock_pinned" | "category" | "scratch";
