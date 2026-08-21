import { Box, Chip, List, ListItemButton, ListItemIcon, ListItemText, Typography } from "@mui/material";
import type { SearchResponse, SearchResult } from "../api/types";
import { fileKey } from "../api/types";
import { KindIcon } from "./KindIcon";
import { Highlight } from "./Highlight";

interface Props {
  data: SearchResponse | null;
  loading: boolean;
  error: string | null;
  selectedKey?: string;
  onSelect: (r: SearchResult) => void;
}

export function SearchResults({ data, loading, error, selectedKey, onSelect }: Props) {
  if (error) return <Typography color="error" sx={{ p: 2 }}>{error}</Typography>;
  if (!data) return null;
  return (
    <Box>
      <Box sx={{ px: 2, py: 0.5, display: "flex", alignItems: "center", gap: 1 }}>
        <Typography variant="caption" color="text.secondary">
          {loading ? "検索中…" : `${data.total} 件`}
        </Typography>
        <Chip size="small" label={data.mode === "path" ? "パス推測" : "ファイル名"} variant="outlined" />
      </Box>
      <List dense disablePadding sx={{ px: 1 }}>
        {data.results.map((r) => {
          const key = fileKey(r);
          const dir = r.rel_path.includes("/") ? r.rel_path.slice(0, r.rel_path.lastIndexOf("/")) : "";
          return (
            <ListItemButton key={key} selected={key === selectedKey} onClick={() => onSelect(r)}>
              <ListItemIcon sx={{ minWidth: 32 }}>
                <KindIcon kind={r.kind} />
              </ListItemIcon>
              <ListItemText
                primary={
                  <span style={{ wordBreak: "break-all" }}>
                    <Highlight text={r.stem} ranges={r.highlights} />
                    <Typography component="span" variant="caption" color="text.disabled">
                      .{r.ext}
                    </Typography>
                  </span>
                }
                secondary={
                  <span style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
                    <Chip size="small" label={r.root} sx={{ height: 18, fontSize: 11 }} />
                    <span style={{ wordBreak: "break-all" }}>{dir || "/"}</span>
                    <Typography component="span" variant="caption" color="text.disabled" title={r.reason}>
                      {r.score}
                    </Typography>
                  </span>
                }
                secondaryTypographyProps={{ component: "div" }}
              />
            </ListItemButton>
          );
        })}
      </List>
      {!loading && data.results.length === 0 && (
        <Typography sx={{ p: 2 }} color="text.secondary">
          該当なし
        </Typography>
      )}
    </Box>
  );
}
