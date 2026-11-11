import { DisplayEngine } from "./display-engine";
import guest from "./display-guest.js?raw";
import type { DisplayJob, DisplayReply } from "./display-model";
const send = (message: DisplayReply) => self.postMessage(message);
let engine: DisplayEngine;
self.onmessage = async ({ data }: MessageEvent<DisplayJob>) => {
  try {
    if (data.type === "init") {
      engine = await DisplayEngine.create(data.input, guest, data.disabled);
      send({ type: "ready" });
    } else {
      const values = engine.evaluate(data.indices, (program) =>
        send({ type: "started", program }),
      );
      send({ type: "rows", indices: data.indices, values });
    }
  } catch (error) {
    send({
      type: "error",
      message: error instanceof Error ? error.message : String(error),
    });
  }
};
