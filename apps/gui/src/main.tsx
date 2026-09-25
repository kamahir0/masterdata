import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { bootstrapApplicationUserState } from "./user-state";
import "./styles.css";

async function bootstrap() {
  const initialState = await bootstrapApplicationUserState();

  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App initialState={initialState} />
    </StrictMode>,
  );
}

bootstrap().catch((error) => {
  console.error("Failed to bootstrap application user state, falling back to default:", error);
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});


