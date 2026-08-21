import { Box, IconButton, Stack, Tooltip, Typography } from "@mui/material";
import OpenInNewIcon from "@mui/icons-material/OpenInNew";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import RefreshIcon from "@mui/icons-material/Refresh";
import { useState } from "react";
import type { FileKind } from "../../api/types";
import { fileUrl, isTauri, openExternal, revealInFolder } from "../../api/client";
import { HtmlViewer } from "./HtmlViewer";
import { MarkdownViewer } from "./MarkdownViewer";
import { MermaidViewer } from "./MermaidViewer";
import { KindIcon } from "../KindIcon";

export function kindOf(rel: string): FileKind {
  const ext = rel.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "html" || ext === "htm") return "html";
  if (ext === "md" || ext === "markdown") return "markdown";
  if (ext === "mmd" || ext === "mermaid") return "mermaid";
  return "other";
}

interface Props {
  root: string;
  rel: string;
  onNavigate: (root: string, rel: string) => void;
}

export function Viewer({ root, rel, onNavigate }: Props) {
  const kind = kindOf(rel);
  const [nonce, setNonce] = useState(0);
  const copyRef = async () => {
    await navigator.clipboard.writeText(`file:${root}/${rel}`);
  };
  return (
    <Box sx={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <Stack direction="row" alignItems="center" spacing={1} sx={{ px: 2, py: 0.5, borderBottom: 1, borderColor: "divider" }}>
        <KindIcon kind={kind} />
        <Typography variant="body2" noWrap sx={{ flex: 1 }} title={`${root}/${rel}`}>
          <Typography component="span" variant="body2" color="text.secondary">
            {root} /{" "}
          </Typography>
          {rel}
        </Typography>
        <Tooltip title="file: 形式の参照をコピー">
          <IconButton size="small" onClick={copyRef}>
            <ContentCopyIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        <Tooltip title="再読み込み">
          <IconButton size="small" onClick={() => setNonce((n) => n + 1)}>
            <RefreshIcon fontSize="small" />
          </IconButton>
        </Tooltip>
        {isTauri() && (
          <Tooltip title="フォルダを開く">
            <IconButton size="small" onClick={() => revealInFolder(root, rel)}>
              <FolderOpenIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        )}
        <Tooltip title="ブラウザで開く">
          <IconButton size="small" onClick={async () => openExternal(await fileUrl(root, rel))}>
            <OpenInNewIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Stack>
      <Box sx={{ flex: 1, minHeight: 0, overflow: kind === "html" ? "hidden" : "auto" }} key={`${root}/${rel}/${nonce}`}>
        {kind === "html" && <HtmlViewer root={root} rel={rel} />}
        {kind === "markdown" && <MarkdownViewer root={root} rel={rel} onNavigate={onNavigate} />}
        {kind === "mermaid" && <MermaidViewer root={root} rel={rel} />}
        {kind === "other" && <HtmlViewer root={root} rel={rel} />}
      </Box>
    </Box>
  );
}
