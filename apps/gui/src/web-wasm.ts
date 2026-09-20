type WebExports = {
  memory: WebAssembly.Memory;
  md_alloc: (length: number) => number;
  md_free: (pointer: number, length: number) => void;
  md_call: (pointer: number, length: number) => bigint;
};

let loaded: Promise<WebExports> | null = null;

async function wasm(): Promise<WebExports> {
  loaded ??= fetch(`${import.meta.env.BASE_URL}masterdata_web.wasm`)
    .then(async (response) => {
      if (!response.ok) throw new Error(`Could not load shared Rust semantics (${response.status}).`);
      const bytes = await response.arrayBuffer();
      const module = await WebAssembly.instantiate(bytes, {});
      return module.instance.exports as unknown as WebExports;
    });
  return loaded;
}

export async function callRust<T>(request: unknown): Promise<T> {
  const exports = await wasm();
  const input = new TextEncoder().encode(JSON.stringify(request));
  const inputPointer = exports.md_alloc(input.length);
  new Uint8Array(exports.memory.buffer, inputPointer, input.length).set(input);
  const packed = exports.md_call(inputPointer, input.length);
  const outputPointer = Number(packed & 0xffff_ffffn);
  const outputLength = Number(packed >> 32n);
  try {
    const output = new Uint8Array(exports.memory.buffer, outputPointer, outputLength).slice();
    const response = JSON.parse(new TextDecoder().decode(output));
    if (response.error) throw response.error;
    return response as T;
  } finally {
    exports.md_free(outputPointer, outputLength);
  }
}
