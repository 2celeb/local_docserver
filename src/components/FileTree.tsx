import { useMemo } from "react";
import { Box, Typography } from "@mui/material";
import { SimpleTreeView } from "@mui/x-tree-view/SimpleTreeView";
import { TreeItem } from "@mui/x-tree-view/TreeItem";
import FolderIcon from "@mui/icons-material/Folder";
import SourceIcon from "@mui/icons-material/Source";
import type { FileEntry } from "../api/types";
import { fileKey } from "../api/types";
import { KindIcon } from "./KindIcon";

interface Node {
  id: string;
  label: string;
  children: Map<string, Node>;
  entry?: FileEntry;
}

function buildTree(files: FileEntry[]): Node[] {
  const roots = new Map<string, Node>();
  for (const f of files) {
    let node: Node | undefined = roots.get(f.root);
    if (!node) {
      node = { id: `root:${f.root}`, label: f.root, children: new Map() };
      roots.set(f.root, node);
    }
    const parts = f.rel_path.split("/");
    let acc = f.root;
    for (let i = 0; i < parts.length; i++) {
      acc += "/" + parts[i];
      const isLeaf = i === parts.length - 1;
      let child: Node | undefined = node!.children.get(parts[i]);
      if (!child) {
        child = { id: isLeaf ? fileKey(f) : `dir:${acc}`, label: parts[i], children: new Map(), entry: isLeaf ? f : undefined };
        node!.children.set(parts[i], child);
      }
      node = child;
    }
  }
  return [...roots.values()];
}

const sortNodes = (nodes: Node[]) =>
  nodes.sort((a, b) => {
    const ad = a.entry ? 1 : 0;
    const bd = b.entry ? 1 : 0;
    return ad - bd || a.label.localeCompare(b.label, "ja");
  });

function renderNode(n: Node, depth: number): JSX.Element {
  const kids = sortNodes([...n.children.values()]);
  const icon = n.entry ? (
    <KindIcon kind={n.entry.kind} />
  ) : depth === 0 ? (
    <SourceIcon fontSize="small" color="primary" />
  ) : (
    <FolderIcon fontSize="small" sx={{ color: "text.secondary" }} />
  );
  return (
    <TreeItem
      key={n.id}
      itemId={n.id}
      label={
        <Box sx={{ display: "flex", alignItems: "center", gap: 0.75, py: 0.25 }}>
          {icon}
          <Typography variant="body2" noWrap title={n.label} sx={{ fontWeight: depth === 0 ? 600 : 400 }}>
            {n.label}
          </Typography>
          {!n.entry && depth === 0 && (
            <Typography variant="caption" color="text.disabled">
              {countLeaves(n)}
            </Typography>
          )}
        </Box>
      }
    >
      {kids.map((k) => renderNode(k, depth + 1))}
    </TreeItem>
  );
}

function countLeaves(n: Node): number {
  if (n.entry) return 1;
  let c = 0;
  for (const k of n.children.values()) c += countLeaves(k);
  return c;
}

interface Props {
  files: FileEntry[];
  selectedKey?: string;
  onSelect: (f: FileEntry) => void;
}

export function FileTree({ files, selectedKey, onSelect }: Props) {
  const tree = useMemo(() => sortNodes(buildTree(files)), [files]);
  const byId = useMemo(() => new Map(files.map((f) => [fileKey(f), f])), [files]);
  const defaultExpanded = useMemo(() => tree.map((t) => t.id), [tree]);
  if (!files.length)
    return (
      <Typography color="text.secondary" sx={{ p: 2 }}>
        ファイルがありません。設定からディレクトリを追加してください。
      </Typography>
    );
  return (
    <SimpleTreeView
      defaultExpandedItems={defaultExpanded}
      selectedItems={selectedKey ?? null}
      onSelectedItemsChange={(_, id) => {
        const f = id ? byId.get(id) : undefined;
        if (f) onSelect(f);
      }}
      sx={{ px: 1, "& .MuiTreeItem-content": { py: 0 } }}
    >
      {tree.map((n) => renderNode(n, 0))}
    </SimpleTreeView>
  );
}
