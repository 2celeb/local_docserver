import { forwardRef } from "react";
import { IconButton, InputAdornment, MenuItem, Select, TextField, Tooltip } from "@mui/material";
import SearchIcon from "@mui/icons-material/Search";
import ClearIcon from "@mui/icons-material/Clear";
import type { SearchMode } from "../api/types";

interface Props {
  value: string;
  onChange: (v: string) => void;
  mode: SearchMode;
  onModeChange: (m: SearchMode) => void;
  onSubmit?: () => void;
}

export const SearchBar = forwardRef<HTMLInputElement, Props>(function SearchBar(
  { value, onChange, mode, onModeChange, onSubmit },
  ref,
) {
  return (
    <TextField
      inputRef={ref}
      size="small"
      fullWidth
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") onSubmit?.();
        if (e.key === "Escape") onChange("");
      }}
      placeholder="ファイル名、または file:repo/dir/name.html を貼り付け (Ctrl+K)"
      slotProps={{
        input: {
          startAdornment: (
            <InputAdornment position="start">
              <SearchIcon fontSize="small" />
            </InputAdornment>
          ),
          endAdornment: (
            <InputAdornment position="end">
              {value && (
                <IconButton size="small" onClick={() => onChange("")} aria-label="clear">
                  <ClearIcon fontSize="small" />
                </IconButton>
              )}
              <Tooltip title="検索モード: auto は入力形状から自動判定">
                <Select
                  variant="standard"
                  disableUnderline
                  value={mode}
                  onChange={(e) => onModeChange(e.target.value as SearchMode)}
                  sx={{ fontSize: 12, ml: 1, minWidth: 64 }}
                >
                  <MenuItem value="auto">auto</MenuItem>
                  <MenuItem value="path">path</MenuItem>
                  <MenuItem value="name">name</MenuItem>
                </Select>
              </Tooltip>
            </InputAdornment>
          ),
        },
      }}
    />
  );
});
