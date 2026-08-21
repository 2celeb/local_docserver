/**
 * バックエンド呼び出し層。
 * Tauri 内では IPC (invoke) を優先し、ブラウザ直アクセス時は HTTP API にフォールバックする。
 * HTML の iframe 表示と raw 取得は常に HTTP サーバ (/r, /api/raw) を使う。
 */
import type { FileEntry, RootInfo, SearchMode, SearchResponse, Settings } from "./types";
import { encodePath } from "./types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isTauri = () => typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

let serverBase: string | null = null;

/** HTTP サーバのベース URL（末尾スラッシュ無し） */
export async function getServerBase(): Promise<string> {
  if (serverBase) return serverBase;
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    const url = await invoke<string | null>("server_url");
    serverBase = (url ?? "").replace(/\/$/, "");
  } else {
    serverBase = "";
  }
  return serverBase;
}

async function getJson<T>(path: string): Promise<T> {
  const base = await getServerBase();
  const r = await fetch(base + path);
  if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
  return r.json();
}

export async function search(q: string, mode: SearchMode = "auto", limit = 50): Promise<SearchResponse> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<SearchResponse>("search", { q, mode, limit });
  }
  const p = new URLSearchParams({ q, mode, limit: String(limit) });
  return getJson(`/api/search?${p}`);
}

export async function listFiles(root?: string): Promise<FileEntry[]> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<FileEntry[]>("list_files", { root: root ?? null });
  }
  const p = root ? `?root=${encodeURIComponent(root)}` : "";
  return (await getJson<{ files: FileEntry[] }>(`/api/files${p}`)).files;
}

export async function listRoots(): Promise<RootInfo[]> {
  return (await getJson<{ roots: RootInfo[] }>("/api/roots")).roots;
}

export async function getSettings(): Promise<Settings> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<Settings>("get_settings");
  }
  return (await getJson<{ settings: Settings }>("/api/config")).settings;
}

export async function updateSettings(settings: Settings): Promise<number> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<number>("update_settings", { settings });
  }
  const base = await getServerBase();
  const r = await fetch(base + "/api/config", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(settings),
  });
  if (!r.ok) throw new Error(await r.text());
  return (await r.json()).count;
}

export async function reload(): Promise<number> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<number>("reload");
  }
  const base = await getServerBase();
  const r = await fetch(base + "/api/reload", { method: "POST" });
  return (await r.json()).count;
}

export async function fetchRaw(root: string, rel: string): Promise<string> {
  const base = await getServerBase();
  const p = new URLSearchParams({ root, path: rel });
  const r = await fetch(`${base}/api/raw?${p}`);
  if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
  return r.text();
}

/** `/r/<root>/<path>` の配信 URL */
export async function fileUrl(root: string, rel: string): Promise<string> {
  const base = await getServerBase();
  return `${base}/r/${encodeURIComponent(root)}/${encodePath(rel)}`;
}

export async function openExternal(url: string) {
  if (isTauri()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener");
  }
}

export async function revealInFolder(root: string, rel: string) {
  if (!isTauri()) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("reveal", { root, path: rel });
}

export async function pickDirectory(): Promise<string | null> {
  if (!isTauri()) return window.prompt("ディレクトリの絶対パスを入力してください");
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ directory: true, multiple: false });
  return typeof r === "string" ? r : null;
}
