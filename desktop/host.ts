import { createInterface } from "node:readline";
import { ProjectService } from "./service";
import { encode, decode } from "../src/core/wire";
const service = new ProjectService();
const lines = createInterface({ input: process.stdin });
lines.on("line", (line) => {
  let id: number;
  try {
    const request = JSON.parse(line);
    id = request.id;
    void service.request(request.method, decode(request.payload)).then(
      (value) =>
        process.stdout.write(
          JSON.stringify({ id, payload: encode(value) }) + "\n",
        ),
      (error) =>
        process.stdout.write(
          JSON.stringify({
            id,
            error: error instanceof Error ? error.message : String(error),
          }) + "\n",
        ),
    );
  } catch (error) {
    process.stderr.write(`Invalid native request: ${String(error)}\n`);
  }
});
// A host never outlives its owning Tauri process. Ongoing .NET children are stopped below.
lines.on("close", () => {
  process.emit("SIGTERM");
  process.exit(0);
});
