import { validate } from "./validation";
self.onmessage = ({ data }) => {
  try {
    self.postMessage({
      revision: data.revision,
      problems: validate(data.snapshot),
    });
  } catch (error) {
    self.postMessage({
      revision: data.revision,
      problems: [
        { source: "", path: [], code: "VALIDATION", message: String(error) },
      ],
    });
  }
};
