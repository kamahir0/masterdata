import "./desktop";
import React from "react";
import { createRoot } from "react-dom/client";
import "antd/dist/reset.css";
import "./style.css";
import Root from "./App";
createRoot(document.getElementById("root")!).render(<Root />);
