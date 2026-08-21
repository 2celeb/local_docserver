import { useEffect, useRef, useState } from "react";
import { search } from "../api/client";
import type { SearchMode, SearchResponse } from "../api/types";

/** 入力を debounce して検索。最後に投げたリクエストの結果だけを採用する。 */
export function useSearch(query: string, mode: SearchMode, delay = 180) {
  const [data, setData] = useState<SearchResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const seq = useRef(0);

  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setData(null);
      setLoading(false);
      return;
    }
    const id = ++seq.current;
    setLoading(true);
    const t = setTimeout(async () => {
      try {
        const r = await search(q, mode, 100);
        if (id === seq.current) {
          setData(r);
          setError(null);
        }
      } catch (e) {
        if (id === seq.current) setError(String(e));
      } finally {
        if (id === seq.current) setLoading(false);
      }
    }, delay);
    return () => clearTimeout(t);
  }, [query, mode, delay]);

  return { data, loading, error };
}
