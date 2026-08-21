import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import {
  AppBar,
  Box,
  Chip,
  CssBaseline,
  Divider,
  IconButton,
  Snackbar,
  Tab,
  Tabs,
  ThemeProvider,
  Toolbar,
  Tooltip,
  Typography,
} from "@mui/material";
import SettingsIcon from "@mui/icons-material/Settings";
import RefreshIcon from "@mui/icons-material/Refresh";
import DarkModeIcon from "@mui/icons-material/DarkMode";
import LightModeIcon from "@mui/icons-material/LightMode";
import LinkIcon from "@mui/icons-material/Link";
import MenuOpenIcon from "@mui/icons-material/MenuOpen";
import { makeTheme } from "./theme";
import { SearchBar } from "./components/SearchBar";
import { SearchResults } from "./components/SearchResults";
import { FileTree } from "./components/FileTree";
import { Viewer } from "./components/viewers/Viewer";
import { SettingsDialog } from "./components/SettingsDialog";
import { useSearch } from "./hooks/useSearch";
import { useLocalStorage } from "./hooks/useLocalStorage";
import { getServerBase, listFiles, listRoots, reload } from "./api/client";
import type { FileEntry, RootInfo, SearchMode } from "./api/types";
import { encodePath } from "./api/types";

/** URL `/view/<root>/<rel...>` を解析 */
function parseViewPath(pathname: string): { root: string; rel: string } | null {
  const m = pathname.match(/^\/view\/([^/]+)\/(.+)$/);
  if (!m) return null;
  return { root: decodeURIComponent(m[1]), rel: m[2].split("/").map(decodeURIComponent).join("/") };
}

export default function App() {
  const [mode, setMode] = useLocalStorage<"light" | "dark">("theme", "light");
  const theme = useMemo(() => makeTheme(mode), [mode]);
  const navigate = useNavigate();
  const location = useLocation();
  const current = useMemo(() => parseViewPath(location.pathname), [location.pathname]);

  const [query, setQuery] = useState("");
  const [searchMode, setSearchMode] = useState<SearchMode>("auto");
  const { data, loading, error } = useSearch(query, searchMode);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [roots, setRoots] = useState<RootInfo[]>([]);
  const [tab, setTab] = useState<0 | 1>(0);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [snack, setSnack] = useState<string | null>(null);
  const [serverUrl, setServerUrl] = useState("");
  const [sidebar, setSidebar] = useState(true);
  const [sidebarWidth, setSidebarWidth] = useLocalStorage("sidebarWidth", 380);
  const inputRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    try {
      const [f, r] = await Promise.all([listFiles(), listRoots()]);
      setFiles(f);
      setRoots(r);
    } catch (e) {
      setSnack(`サーバに接続できません: ${e}`);
    }
  }, []);

  useEffect(() => {
    refresh();
    getServerBase().then((b) => setServerUrl(b || window.location.origin));
  }, [refresh]);

  // 検索入力があれば結果タブへ、空ならツリーへ
  useEffect(() => {
    setTab(query.trim() ? 0 : 1);
  }, [query]);

  // 確信度の高いパス推測は即オープン
  useEffect(() => {
    if (data?.best && data.mode === "path") {
      const r = data.results[0];
      navigate(`/view/${encodeURIComponent(r.root)}/${encodePath(r.rel_path)}`);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.best]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const open = useCallback(
    (root: string, rel: string) => navigate(`/view/${encodeURIComponent(root)}/${encodePath(rel)}`),
    [navigate],
  );

  const selectedKey = current ? `${current.root}/${current.rel}` : undefined;

  const onReload = async () => {
    const n = await reload();
    await refresh();
    setSnack(`再スキャン完了: ${n} ファイル`);
  };

  // リサイズハンドル
  const dragging = useRef(false);
  useEffect(() => {
    const mv = (e: MouseEvent) => dragging.current && setSidebarWidth(Math.min(700, Math.max(240, e.clientX)));
    const up = () => (dragging.current = false);
    window.addEventListener("mousemove", mv);
    window.addEventListener("mouseup", up);
    return () => {
      window.removeEventListener("mousemove", mv);
      window.removeEventListener("mouseup", up);
    };
  }, [setSidebarWidth]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Box sx={{ display: "flex", flexDirection: "column", height: "100vh" }}>
        <AppBar position="static" color="default" elevation={1}>
          <Toolbar variant="dense" sx={{ gap: 1 }}>
            <IconButton size="small" onClick={() => setSidebar((s) => !s)} title="サイドバー切替">
              <MenuOpenIcon />
            </IconButton>
            <Typography variant="h6" sx={{ mr: 2, whiteSpace: "nowrap", cursor: "pointer" }} onClick={() => navigate("/")}>
              Local DocServer
            </Typography>
            <Box sx={{ flex: 1, maxWidth: 760 }}>
              <SearchBar ref={inputRef} value={query} onChange={setQuery} mode={searchMode} onModeChange={setSearchMode} />
            </Box>
            <Box sx={{ flex: 1 }} />
            <Tooltip title={`サーバ URL をコピー: ${serverUrl}`}>
              <Chip
                icon={<LinkIcon />}
                size="small"
                label={serverUrl.replace(/^https?:\/\//, "")}
                onClick={() => navigator.clipboard.writeText(serverUrl).then(() => setSnack("URL をコピーしました"))}
                variant="outlined"
              />
            </Tooltip>
            <IconButton size="small" onClick={onReload} title="再スキャン">
              <RefreshIcon />
            </IconButton>
            <IconButton size="small" onClick={() => setMode(mode === "light" ? "dark" : "light")} title="テーマ切替">
              {mode === "light" ? <DarkModeIcon /> : <LightModeIcon />}
            </IconButton>
            <IconButton size="small" onClick={() => setSettingsOpen(true)} title="設定">
              <SettingsIcon />
            </IconButton>
          </Toolbar>
        </AppBar>

        <Box sx={{ display: "flex", flex: 1, minHeight: 0 }}>
          {sidebar && (
            <>
              <Box sx={{ width: sidebarWidth, flexShrink: 0, display: "flex", flexDirection: "column", bgcolor: "background.paper" }}>
                <Tabs value={tab} onChange={(_, v) => setTab(v)} variant="fullWidth" sx={{ minHeight: 36 }}>
                  <Tab label="検索結果" sx={{ minHeight: 36, py: 0 }} />
                  <Tab label={`ファイル (${files.length})`} sx={{ minHeight: 36, py: 0 }} />
                </Tabs>
                <Divider />
                <Box sx={{ flex: 1, overflow: "auto" }}>
                  {tab === 0 ? (
                    query.trim() ? (
                      <SearchResults data={data} loading={loading} error={error} selectedKey={selectedKey} onSelect={(r) => open(r.root, r.rel_path)} />
                    ) : (
                      <Box sx={{ p: 2, color: "text.secondary" }}>
                        <Typography variant="body2" gutterBottom>
                          ファイル名の一部、または <code>file:repo/dir/name.html</code> のようなパスを入力してください。
                        </Typography>
                        <Typography variant="caption">
                          ルート: {roots.map((r) => `${r.name} (${r.count})`).join(" / ") || "未設定"}
                        </Typography>
                      </Box>
                    )
                  ) : (
                    <FileTree files={files} selectedKey={selectedKey} onSelect={(f) => open(f.root, f.rel_path)} />
                  )}
                </Box>
              </Box>
              <Box
                onMouseDown={() => (dragging.current = true)}
                sx={{ width: 5, cursor: "col-resize", bgcolor: "divider", "&:hover": { bgcolor: "primary.main" } }}
              />
            </>
          )}
          <Box sx={{ flex: 1, minWidth: 0, bgcolor: "background.default" }}>
            {current ? (
              <Viewer root={current.root} rel={current.rel} onNavigate={open} />
            ) : (
              <Box sx={{ p: 6, color: "text.secondary", textAlign: "center" }}>
                <Typography variant="h5" gutterBottom>
                  ドキュメントを選択してください
                </Typography>
                <Typography variant="body2">
                  左のツリーから選ぶか、上の検索ボックスにファイル名 / パスを入力します（Ctrl+K）。
                </Typography>
              </Box>
            )}
          </Box>
        </Box>
      </Box>

      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSaved={async (n) => {
          await refresh();
          setSnack(`設定を保存しました: ${n} ファイル`);
        }}
      />
      <Snackbar open={!!snack} autoHideDuration={3000} onClose={() => setSnack(null)} message={snack} />
    </ThemeProvider>
  );
}
