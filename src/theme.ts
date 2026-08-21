import { createTheme } from "@mui/material/styles";

export const makeTheme = (mode: "light" | "dark") =>
  createTheme({
    palette: {
      mode,
      primary: { main: mode === "light" ? "#1565c0" : "#90caf9" },
      secondary: { main: "#00897b" },
      background: mode === "light" ? { default: "#f5f6f8", paper: "#ffffff" } : { default: "#121212", paper: "#1e1e1e" },
    },
    typography: {
      fontFamily: ['"Roboto"', '"Noto Sans JP"', '"Hiragino Sans"', '"Yu Gothic"', "Meiryo", "sans-serif"].join(","),
      fontSize: 13,
    },
    components: {
      MuiListItemButton: { styleOverrides: { root: { borderRadius: 6 } } },
    },
  });
