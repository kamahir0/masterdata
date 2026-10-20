import { parse, stringify } from "smol-toml";
import { isDeepStrictEqual } from "node:util";
import { defaultConfig, type Config } from "../src/core/types";

export const projectFileName = "masterdata.toml";
const table = (value: unknown): value is Record<string, any> =>
  !!value &&
  typeof value === "object" &&
  !Array.isArray(value) &&
  [Object.prototype, null].includes(Object.getPrototypeOf(value));
function invalid(key: string): never {
  throw new Error(`プロジェクト設定の ${key} の形式が不正です。`);
}

export function readConfig(text: string): Config {
  const data = parse(text);
  if (typeof data.name !== "string" || !data.name.trim())
    invalid("name（空でない文字列）");
  const config = defaultConfig(data.name);
  if (data.profiles !== undefined) {
    if (!table(data.profiles)) invalid("profiles（名前付きの設定）");
    config.profiles = Object.fromEntries(
      Object.entries(data.profiles).map(([name, value]) => {
        if (!name.trim() || !table(value)) invalid(`profiles.${name}`);
        const profile = { include: [] as string[], exclude: [] as string[] };
        for (const key of ["include", "exclude"] as const) {
          const tags = value[key];
          if (tags !== undefined) {
            if (
              !Array.isArray(tags) ||
              !tags.every((v) => typeof v === "string")
            )
              invalid(`profiles.${name}.${key}（文字列の配列）`);
            profile[key] = tags;
          }
        }
        return [name, profile];
      }),
    );
  }
  if (data.publish !== undefined) {
    if (!table(data.publish)) invalid("publish");
    const publish = data.publish as Record<string, unknown>;
    for (const key of ["csharp", "binary"] as const) {
      const target = publish[key];
      if (target !== undefined) {
        if (typeof target !== "string") invalid(`publish.${key}（文字列）`);
        config.publish[key] = target;
      }
    }
  }
  return config;
}

export function writeConfig(config: Config, before?: string): string {
  if (before !== undefined && isDeepStrictEqual(readConfig(before), config))
    return before;
  // A source-only Save must not rewrite project text. When settings do change,
  // retain unknown values, but serialize the app-managed document consistently.
  const previous = before === undefined ? {} : parse(before);
  const output =
    stringify({
      ...previous,
      name: config.name,
      publish: { ...(previous.publish as object), ...config.publish },
      profiles: Object.fromEntries(
        Object.entries(config.profiles).map(([name, value]) => [
          name,
          { ...(previous.profiles as any)?.[name], ...value },
        ]),
      ),
    });
  readConfig(output);
  return output;
}
