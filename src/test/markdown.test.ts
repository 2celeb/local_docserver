import { describe, expect, it } from "vitest";
import { resolveRelative } from "../components/viewers/MarkdownViewer";
import { kindOf } from "../components/viewers/Viewer";

describe("resolveRelative", () => {
  it("resolves sibling and parent", () => {
    expect(resolveRelative("docs/a/b.md", "c.md")).toBe("docs/a/c.md");
    expect(resolveRelative("docs/a/b.md", "../x.html")).toBe("docs/x.html");
    expect(resolveRelative("README.md", "docs/flow.mmd")).toBe("docs/flow.mmd");
  });
});

describe("kindOf", () => {
  it("maps extensions", () => {
    expect(kindOf("a/b.HTML")).toBe("html");
    expect(kindOf("x.markdown")).toBe("markdown");
    expect(kindOf("x.mmd")).toBe("mermaid");
    expect(kindOf("x.png")).toBe("other");
  });
});
