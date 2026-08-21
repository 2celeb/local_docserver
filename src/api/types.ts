export type FileKind = "html" | "markdown" | "mermaid" | "other";

export interface FileEntry {
  root: string;
  rel_path: string;
  file_name: string;
  stem: string;
  ext: string;
  kind: FileKind;
  size: number;
  modified: number;
}

export interface SearchResult extends FileEntry {
  score: number;
  reason: string;
  highlights: [number, number][];
}

export type SearchMode = "auto" | "path" | "name";

export interface SearchResponse {
  query: string;
  mode: SearchMode;
  total: number;
  results: SearchResult[];
  best: string | null;
}

export interface RootInfo {
  name: string;
  path: string;
  exists: boolean;
  count: number;
}

export interface RootSettings {
  name: string;
  path: string;
  exclude: string[];
}

export interface Settings {
  server: { host: string; port: number; open_browser: boolean };
  roots: RootSettings[];
  include_extensions: string[];
  respect_gitignore: boolean;
  watch: boolean;
  max_depth: number;
}

export const fileKey = (f: { root: string; rel_path: string }) => `${f.root}/${f.rel_path}`;

export const encodePath = (rel: string) => rel.split("/").map(encodeURIComponent).join("/");
