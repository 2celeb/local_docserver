import { useEffect, useState } from "react";
import { Box, Button, CircularProgress, Paper, Stack, ToggleButton, ToggleButtonGroup } from "@mui/material";
import DownloadIcon from "@mui/icons-material/Download";
import { fetchRaw } from "../../api/client";
import { MermaidBlock } from "./MermaidBlock";

/** .mmd / .mermaid 単体ファイルのビューア（図 / ソース切替、SVG 保存） */
export function MermaidViewer({ root, rel }: { root: string; rel: string }) {
  const [code, setCode] = useState<string | null>(null);
  const [view, setView] = useState<"diagram" | "source">("diagram");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setCode(null);
    fetchRaw(root, rel).then(setCode).catch((e) => setError(String(e)));
  }, [root, rel]);

  const download = () => {
    const svg = document.querySelector<SVGElement>("#mermaid-viewer svg");
    if (!svg) return;
    const blob = new Blob([svg.outerHTML], { type: "image/svg+xml" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = rel.split("/").pop()!.replace(/\.\w+$/, "") + ".svg";
    a.click();
    URL.revokeObjectURL(a.href);
  };

  if (error) return <Box sx={{ p: 2, color: "error.main" }}>{error}</Box>;
  if (code === null) return <CircularProgress sx={{ m: 4 }} />;
  return (
    <Box sx={{ p: 2, height: "100%", overflow: "auto" }}>
      <Stack direction="row" spacing={1} sx={{ mb: 1 }}>
        <ToggleButtonGroup size="small" exclusive value={view} onChange={(_, v) => v && setView(v)}>
          <ToggleButton value="diagram">図</ToggleButton>
          <ToggleButton value="source">ソース</ToggleButton>
        </ToggleButtonGroup>
        <Button size="small" startIcon={<DownloadIcon />} onClick={download}>
          SVG 保存
        </Button>
      </Stack>
      <Paper variant="outlined" sx={{ p: 2 }} id="mermaid-viewer">
        {view === "diagram" ? (
          <MermaidBlock code={code} />
        ) : (
          <Box component="pre" sx={{ m: 0, fontSize: 13, whiteSpace: "pre-wrap" }}>
            {code}
          </Box>
        )}
      </Paper>
    </Box>
  );
}
