import { useEffect, useId, useRef, useState } from "react";
import { Alert, Box } from "@mui/material";
import { useTheme } from "@mui/material/styles";

let initialized: "light" | "dark" | null = null;

async function getMermaid(mode: "light" | "dark") {
  const m = (await import("mermaid")).default;
  if (initialized !== mode) {
    m.initialize({
      startOnLoad: false,
      theme: mode === "dark" ? "dark" : "default",
      securityLevel: "loose",
      fontFamily: "Roboto, 'Noto Sans JP', sans-serif",
    });
    initialized = mode;
  }
  return m;
}

/** Mermaid ソースを SVG に描画する。Markdown 内の ```mermaid と .mmd ファイル両方で使用。 */
export function MermaidBlock({ code }: { code: string }) {
  const theme = useTheme();
  const id = useId().replace(/[^a-zA-Z0-9]/g, "");
  const ref = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const m = await getMermaid(theme.palette.mode);
        const { svg, bindFunctions } = await m.render(`mmd-${id}-${Math.random().toString(36).slice(2)}`, code);
        if (!alive || !ref.current) return;
        ref.current.innerHTML = svg;
        bindFunctions?.(ref.current);
        setError(null);
      } catch (e) {
        if (alive) setError(String(e));
      }
    })();
    return () => {
      alive = false;
    };
  }, [code, id, theme.palette.mode]);

  return (
    <Box sx={{ my: 1, overflowX: "auto" }}>
      {error && (
        <Alert severity="error" sx={{ mb: 1, whiteSpace: "pre-wrap", fontFamily: "monospace", fontSize: 12 }}>
          {error}
        </Alert>
      )}
      <Box ref={ref} sx={{ "& svg": { maxWidth: "100%", height: "auto" } }} />
      {error && (
        <Box component="pre" sx={{ fontSize: 12, p: 1, bgcolor: "action.hover", borderRadius: 1 }}>
          {code}
        </Box>
      )}
    </Box>
  );
}
