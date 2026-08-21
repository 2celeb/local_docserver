import HtmlIcon from "@mui/icons-material/Html";
import ArticleIcon from "@mui/icons-material/Article";
import AccountTreeIcon from "@mui/icons-material/AccountTree";
import InsertDriveFileIcon from "@mui/icons-material/InsertDriveFile";
import type { FileKind } from "../api/types";

export function KindIcon({ kind, fontSize = "small" }: { kind: FileKind; fontSize?: "small" | "medium" | "inherit" }) {
  switch (kind) {
    case "html":
      return <HtmlIcon fontSize={fontSize} color="warning" />;
    case "markdown":
      return <ArticleIcon fontSize={fontSize} color="primary" />;
    case "mermaid":
      return <AccountTreeIcon fontSize={fontSize} color="secondary" />;
    default:
      return <InsertDriveFileIcon fontSize={fontSize} />;
  }
}
