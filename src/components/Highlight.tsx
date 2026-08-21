import { Box } from "@mui/material";

/** stem を highlights ([start,end) char index) に従って強調表示 */
export function Highlight({ text, ranges }: { text: string; ranges: [number, number][] }) {
  if (!ranges.length) return <>{text}</>;
  const chars = Array.from(text);
  const marks = new Array<boolean>(chars.length).fill(false);
  for (const [s, e] of ranges) for (let i = s; i < Math.min(e, chars.length); i++) marks[i] = true;
  const parts: { t: string; m: boolean }[] = [];
  for (let i = 0; i < chars.length; i++) {
    const last = parts[parts.length - 1];
    if (last && last.m === marks[i]) last.t += chars[i];
    else parts.push({ t: chars[i], m: marks[i] });
  }
  return (
    <>
      {parts.map((p, i) =>
        p.m ? (
          <Box key={i} component="mark" sx={{ bgcolor: "warning.light", color: "inherit", px: 0, borderRadius: 0.5 }}>
            {p.t}
          </Box>
        ) : (
          <span key={i}>{p.t}</span>
        ),
      )}
    </>
  );
}
