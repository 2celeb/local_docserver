import { useEffect, useState } from "react";
import { Box, CircularProgress } from "@mui/material";
import { fileUrl } from "../../api/client";

/**
 * HTML は配信サーバ経由で iframe 表示する。
 * 同一ディレクトリ構造で配信されるため `../assets/style.css` 等の相対参照がそのまま解決される。
 */
export function HtmlViewer({ root, rel }: { root: string; rel: string }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    fileUrl(root, rel).then((u) => alive && setUrl(u));
    return () => {
      alive = false;
    };
  }, [root, rel]);
  if (!url) return <CircularProgress sx={{ m: 4 }} />;
  return (
    <Box
      component="iframe"
      key={url}
      src={url}
      title={rel}
      sandbox="allow-scripts allow-same-origin allow-popups allow-forms allow-modals"
      sx={{ border: 0, width: "100%", height: "100%", bgcolor: "#fff", display: "block" }}
    />
  );
}
