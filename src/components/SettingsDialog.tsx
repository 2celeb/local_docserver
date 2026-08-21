import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import DeleteIcon from "@mui/icons-material/Delete";
import AddIcon from "@mui/icons-material/Add";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import { getSettings, pickDirectory, updateSettings } from "../api/client";
import type { Settings } from "../api/types";

interface Props {
  open: boolean;
  onClose: () => void;
  onSaved: (count: number) => void;
}

export function SettingsDialog({ open, onClose, onSaved }: Props) {
  const [s, setS] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) getSettings().then(setS).catch((e) => setError(String(e)));
  }, [open]);

  const save = async () => {
    if (!s) return;
    setSaving(true);
    setError(null);
    try {
      const count = await updateSettings(s);
      onSaved(count);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const setRoot = (i: number, patch: Partial<Settings["roots"][number]>) =>
    setS((p) => p && { ...p, roots: p.roots.map((r, j) => (j === i ? { ...r, ...patch } : r)) });

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>設定</DialogTitle>
      <DialogContent dividers>
        {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
        {s && (
          <Stack spacing={2}>
            <Typography variant="subtitle2">公開するリポジトリ / ディレクトリ</Typography>
            {s.roots.map((r, i) => (
              <Stack key={i} direction="row" spacing={1} alignItems="flex-start">
                <TextField
                  label="名前"
                  size="small"
                  value={r.name}
                  onChange={(e) => setRoot(i, { name: e.target.value })}
                  sx={{ width: 180 }}
                  helperText="URL: /r/名前/…"
                />
                <TextField
                  label="パス"
                  size="small"
                  fullWidth
                  value={r.path}
                  onChange={(e) => setRoot(i, { path: e.target.value })}
                  helperText="絶対パス、または設定ファイルからの相対パス"
                />
                <IconButton
                  onClick={async () => {
                    const d = await pickDirectory();
                    if (d) setRoot(i, { path: d, name: r.name || d.split(/[\\/]/).filter(Boolean).pop() || "" });
                  }}
                  title="ディレクトリを選択"
                >
                  <FolderOpenIcon />
                </IconButton>
                <TextField
                  label="除外 (カンマ区切り)"
                  size="small"
                  value={r.exclude.join(",")}
                  onChange={(e) =>
                    setRoot(i, { exclude: e.target.value.split(",").map((x) => x.trim()).filter(Boolean) })
                  }
                  sx={{ width: 220 }}
                />
                <IconButton onClick={() => setS({ ...s, roots: s.roots.filter((_, j) => j !== i) })} title="削除">
                  <DeleteIcon />
                </IconButton>
              </Stack>
            ))}
            <Button
              startIcon={<AddIcon />}
              onClick={() => setS({ ...s, roots: [...s.roots, { name: "", path: "", exclude: [] }] })}
              sx={{ alignSelf: "flex-start" }}
            >
              追加
            </Button>

            <Typography variant="subtitle2">走査</Typography>
            <Stack direction="row" spacing={2} alignItems="center">
              <TextField
                label="対象拡張子"
                size="small"
                value={s.include_extensions.join(",")}
                onChange={(e) =>
                  setS({ ...s, include_extensions: e.target.value.split(",").map((x) => x.trim()).filter(Boolean) })
                }
                sx={{ width: 320 }}
              />
              <TextField
                label="最大深さ"
                size="small"
                type="number"
                value={s.max_depth}
                onChange={(e) => setS({ ...s, max_depth: Number(e.target.value) })}
                sx={{ width: 120 }}
              />
              <FormControlLabel
                control={<Checkbox checked={s.respect_gitignore} onChange={(e) => setS({ ...s, respect_gitignore: e.target.checked })} />}
                label=".gitignore を尊重"
              />
              <FormControlLabel
                control={<Checkbox checked={s.watch} onChange={(e) => setS({ ...s, watch: e.target.checked })} />}
                label="変更を監視 (要再起動)"
              />
            </Stack>

            <Typography variant="subtitle2">サーバ (変更は再起動後に反映)</Typography>
            <Stack direction="row" spacing={2}>
              <TextField
                label="ホスト"
                size="small"
                value={s.server.host}
                onChange={(e) => setS({ ...s, server: { ...s.server, host: e.target.value } })}
              />
              <TextField
                label="ポート (0=自動)"
                size="small"
                type="number"
                value={s.server.port}
                onChange={(e) => setS({ ...s, server: { ...s.server, port: Number(e.target.value) } })}
              />
            </Stack>
          </Stack>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>キャンセル</Button>
        <Button variant="contained" onClick={save} disabled={!s || saving}>
          保存して再スキャン
        </Button>
      </DialogActions>
    </Dialog>
  );
}
