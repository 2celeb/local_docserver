import React, { useEffect, useMemo, useState } from "react";
import { Box, CircularProgress, Link } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeRaw from "rehype-raw";
import rehypeSlug from "rehype-slug";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/github.css";
import { fetchRaw, fileUrl } from "../../api/client";
import { MermaidBlock } from "./MermaidBlock";

interface Props {
  root: string;
  rel: string;
  /** 相対リンクをクリックしたときにアプリ内で開く */
  onNavigate?: (root: string, rel: string) => void;
}

/** 相対パスを rel_path 基準で解決する（'/' 区切り） */
export function resolveRelative(base: string, href: string): string {
  const dir = base.includes("/") ? base.slice(0, base.lastIndexOf("/")).split("/") : [];
  const parts = href.split("/");
  for (const p of parts) {
    if (p === "" || p === ".") continue;
    if (p === "..") dir.pop();
    else dir.push(p);
  }
  return dir.join("/");
}

export function MarkdownViewer({ root, rel, onNavigate }: Props) {
  const theme = useTheme();
  const [md, setMd] = useState<string | null>(null);
  const [base, setBase] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setMd(null);
    fetchRaw(root, rel).then(setMd).catch((e) => setError(String(e)));
    fileUrl(root, rel).then((u) => setBase(u.slice(0, u.lastIndexOf("/") + 1)));
  }, [root, rel]);

  const components = useMemo(
    () => ({
      code({ className, children, ...rest }: any) {
        const lang = /language-(\w+)/.exec(className ?? "")?.[1];
        if (lang === "mermaid") return <MermaidBlock code={String(children).trim()} />;
        return (
          <code className={className} {...rest}>
            {children}
          </code>
        );
      },
      a({ href, children, ...rest }: any) {
        const isExternal = /^[a-z]+:/i.test(href ?? "") || href?.startsWith("#");
        if (!isExternal && href && onNavigate) {
          const [path, hash] = href.split("#");
          const target = resolveRelative(rel, decodeURIComponent(path));
          return (
            <Link
              href={href}
              onClick={(e: React.MouseEvent) => {
                e.preventDefault();
                onNavigate(root, target + (hash ? `#${hash}` : ""));
              }}
              {...rest}
            >
              {children}
            </Link>
          );
        }
        return (
          <Link href={href} target={isExternal && !href.startsWith("#") ? "_blank" : undefined} rel="noopener" {...rest}>
            {children}
          </Link>
        );
      },
      img({ src, ...rest }: any) {
        const abs = /^[a-z]+:/i.test(src ?? "") ? src : base + src;
        return <img src={abs} style={{ maxWidth: "100%" }} {...rest} />;
      },
    }),
    [base, rel, root, onNavigate],
  );

  if (error) return <Box sx={{ p: 2, color: "error.main" }}>{error}</Box>;
  if (md === null) return <CircularProgress sx={{ m: 4 }} />;
  return (
    <Box
      className="markdown-body"
      sx={{
        p: 3,
        maxWidth: 960,
        mx: "auto",
        lineHeight: 1.7,
        "& h1,& h2": { borderBottom: 1, borderColor: "divider", pb: 0.5, mt: 3 },
        "& pre": { p: 1.5, borderRadius: 1, overflowX: "auto", bgcolor: theme.palette.mode === "dark" ? "#0d1117" : "#f6f8fa" },
        "& code": { fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace", fontSize: "0.9em" },
        "& :not(pre) > code": { px: 0.5, borderRadius: 0.5, bgcolor: "action.hover" },
        "& table": { borderCollapse: "collapse", my: 2, display: "block", overflowX: "auto" },
        "& th,& td": { border: 1, borderColor: "divider", px: 1.5, py: 0.5 },
        "& th": { bgcolor: "action.hover" },
        "& blockquote": { borderLeft: 4, borderColor: "divider", pl: 2, ml: 0, color: "text.secondary" },
        "& img": { maxWidth: "100%" },
      }}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeRaw, rehypeSlug, [rehypeHighlight, { ignoreMissing: true }]]}
        components={components}
      >
        {md}
      </ReactMarkdown>
    </Box>
  );
}
