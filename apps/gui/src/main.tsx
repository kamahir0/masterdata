import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ConfigProvider, theme } from "antd";
import App from "./App";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm, token: {
      colorPrimary: "#8b7cf7", borderRadius: 8, colorBgContainer: "#171e30",
      colorBgElevated: "#1c2438", fontSize: 13,
      fontFamily: 'Inter, system-ui, -apple-system, "Segoe UI", sans-serif',
    } }}><App /></ConfigProvider>
  </StrictMode>,
);

