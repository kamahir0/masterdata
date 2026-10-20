import fs from "node:fs/promises";
import path from "node:path";
import { projectFileName, writeConfig } from "../desktop/config";
import {
  digest,
  loadWorkspace,
  optionalRead,
  safePath,
  saveWorkspace,
  type Workspace,
} from "../desktop/files";
import { buildWorkspace, publishBuild } from "../desktop/build";
import { Session } from "../src/core/session";
import {
  type BuildResult,
  type Data,
  type LoadedProject,
  defaultConfig,
  isObject,
  tables,
} from "../src/core/types";
import {
  flagBool,
  flagString,
  flagStrings,
  parseArgs,
  parseAssignments,
  parseStructured,
  splitCsv,
  stringifyJson,
} from "./args";
import {
  applyOperation,
  findTable,
  formatProblems,
  inspectProject,
  queryRecords,
} from "./operations";

export interface CliOptions {
  cwd?: string;
  stdin?: string;
}

export interface CliResult {
  exitCode: number;
  stdout: string;
  stderr: string;
  data?: unknown;
}

const HELP_TEXT = `MasterData CLI — 構造を理解するゲームデータ管理・編集・ビルドツール

使い方:
  masterdata <command> [options]

主なコマンド:
  init [dir] [--name <name>]                     プロジェクト（masterdata.toml と sources/）を新規作成
  inspect [target]                               プロジェクト構造・スキーマ・型・空きPKを一覧表示
  validate [--profile <name>]                    構文・型・キー重複・MasterRef参照整合性を検証
  config show | set | profile ...                masterdata.toml の設定・プロファイルを操作
  table create | delete | data-file | column | key | ref ...
                                                 テーブル定義・分離データファイル・列・キー・参照を編集
  type create | delete | set-underlying | member | field ...
                                                 Enum / Flags / Custom / ValueObject 型定義を編集
  record list | get | add | set | delete ...     レコードの検索・追加・パス指定更新・削除
  rename table | type | field | member ...       定義と参照箇所・レコード値を横断して一括リネーム
  source move | delete ...                       YAMLソースファイルの移動・削除
  fix [--table <Table>]                          順序や既存値を保ったまま未設定key・欠落項目補完とYAML正規化
  tx [--file <path>] [--ops <json/yaml>]         複数の編集操作を単一セッションで一括・アトミックに適用
  build [--profile <name>]                       C#コードとMasterMemoryバイナリを生成しreload検証
  publish [--build] [--csharp <dir>] [--binary <file>]
                                                 最新Build成果物を出力先へ安全に配置

共通オプション:
  --project <path>   masterdata.toml またはプロジェクトルートを指定（省略時は親方向へ探索）
  --json             機械可読なJSON形式で出力
  --dry-run          変更系コマンドでファイルへ書き込まず差分と診断結果のみ確認
  --strict           変更適用後に未解決の診断エラーが1件でもあれば保存せず非0終了で中止
`;

export async function resolveProjectFile(
  projectOption?: string,
  cwd = process.cwd(),
): Promise<string> {
  if (projectOption) {
    const resolved = path.resolve(cwd, projectOption);
    let stat;
    try {
      stat = await fs.lstat(resolved);
    } catch {
      throw new Error(`指定されたプロジェクトが見つかりません: ${projectOption}`);
    }
    if (stat.isFile()) {
      if (path.basename(resolved) !== projectFileName)
        throw new Error(
          `${projectFileName} ファイルまたはプロジェクトフォルダを指定してください: ${projectOption}`,
        );
      return resolved;
    }
    if (stat.isDirectory()) {
      const candidate = path.join(resolved, projectFileName);
      const fileStat = await fs.lstat(candidate).catch(() => null);
      if (!fileStat?.isFile())
        throw new Error(
          `指定フォルダ内に ${projectFileName} が見つかりません: ${projectOption}`,
        );
      return candidate;
    }
    throw new Error(`不正なプロジェクト指定です: ${projectOption}`);
  }

  let current = path.resolve(cwd);
  while (true) {
    const candidate = path.join(current, projectFileName);
    const stat = await fs.lstat(candidate).catch(() => null);
    if (stat?.isFile()) return candidate;
    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }
  throw new Error(
    `${projectFileName} が見つかりません。\`masterdata init\` で作成するか \`--project\` を指定してください。`,
  );
}

export function computeWorkspaceDigest(project: LoadedProject): string {
  const parts = [
    `config:${digest(project.configText)}`,
    ...[...project.saved]
      .sort((a, b) => a.path.localeCompare(b.path))
      .map((f) => `source:${f.path}:${digest(f.text)}`),
  ];
  return digest(parts.join("\n"));
}

async function commitSession(
  workspace: Workspace,
  session: Session,
  options: { dryRun?: boolean; strict?: boolean },
) {
  const problems = formatProblems(session.state.snapshot);
  if (options.strict && problems.length > 0) {
    const preview = problems
      .slice(0, 10)
      .map((p) => `  - [${p.code}] ${p.file}: ${p.message}`)
      .join("\n");
    const err: any = new Error(
      `--strict 指定により保存を中止しました（未解決の診断エラー: ${problems.length}件）\n${preview}`,
    );
    err.problems = problems;
    throw err;
  }

  const desiredFiles = session.files();
  const baselineMap = new Map(workspace.baseline.map((f) => [f.path, f.text]));
  const desiredMap = new Map(desiredFiles.map((f) => [f.path, f.text]));
  const changedSources: string[] = [];
  for (const [p, text] of desiredMap) {
    if (baselineMap.get(p) !== text) changedSources.push(`sources/${p}`);
  }
  for (const p of baselineMap.keys()) {
    if (!desiredMap.has(p)) changedSources.push(`sources/${p} (deleted)`);
  }
  const nextConfigText = writeConfig(
    session.state.snapshot.config,
    workspace.configText,
  );
  if (nextConfigText !== workspace.configText)
    changedSources.unshift(projectFileName);

  if (!options.dryRun) {
    await saveWorkspace(workspace, {
      token: workspace.token,
      files: desiredFiles,
      folders: session.state.snapshot.folders,
      config: session.state.snapshot.config,
    });
  }

  return {
    dryRun: !!options.dryRun,
    changedFiles: changedSources,
    problemCount: problems.length,
    ...(problems.length ? { problems: problems.slice(0, 20) } : {}),
  };
}

function buildCliOperation(
  session: Session,
  command: string,
  rest: string[],
  flags: Record<string, (string | boolean)[]>,
): Record<string, any> {
  const snapshot = session.state.snapshot;

  if (command === "config") {
    const sub = rest[0] ?? "show";
    if (sub === "set") {
      return {
        op: "config.set",
        ...(flagString(flags, "name") !== undefined
          ? { name: flagString(flags, "name") }
          : {}),
        ...(flagString(flags, "publish-csharp") !== undefined
          ? { publishCsharp: flagString(flags, "publish-csharp") }
          : {}),
        ...(flagString(flags, "publish-binary") !== undefined
          ? { publishBinary: flagString(flags, "publish-binary") }
          : {}),
      };
    }
    if (sub === "profile") {
      const action = rest[1];
      const name = rest[2] ?? flagString(flags, "name");
      if (action === "set") {
        return {
          op: "config.profile.set",
          name,
          ...(flagString(flags, "include") !== undefined
            ? { include: splitCsv(flagString(flags, "include")) }
            : {}),
          ...(flagString(flags, "exclude") !== undefined
            ? { exclude: splitCsv(flagString(flags, "exclude")) }
            : {}),
        };
      }
      if (action === "remove" || action === "delete") {
        return { op: "config.profile.remove", name };
      }
    }
    throw new Error(`未知の config サブコマンドです: ${ rest.join(" ") }`);
  }

  if (command === "table") {
    const sub = rest[0];
    if (sub === "create") {
      const name = rest[1] ?? flagString(flags, "name");
      const fieldsRaw = flagString(flags, "fields");
      const fields = fieldsRaw
        ? fieldsRaw.trim().startsWith("[")
          ? parseStructured(fieldsRaw)
          : splitCsv(fieldsRaw)
        : undefined;
      return {
        op: "table.create",
        name,
        path: flagString(flags, "path") ?? flagString(flags, "file"),
        pk: flagString(flags, "pk"),
        csharpName: flagString(flags, "csharp-name"),
        ...(fields ? { fields } : {}),
      };
    }
    if (sub === "delete" || sub === "remove") {
      return { op: "table.delete", name: rest[1] ?? flagString(flags, "table") };
    }
    if (sub === "data-file") {
      const action = rest[1];
      if (action === "create" || action === "add") {
        return {
          op: "table.dataFile.create",
          table: rest[2] ?? flagString(flags, "table"),
          path: rest[3] ?? flagString(flags, "path") ?? flagString(flags, "file"),
        };
      }
    }
    if (sub === "column") {
      const action = rest[1];
      const table = rest[2] ?? flagString(flags, "table");
      if (action === "add") {
        return {
          op: "table.column.add",
          table,
          name: rest[3] ?? flagString(flags, "name"),
          type: rest[4] ?? flagString(flags, "type"),
          array: flagBool(flags, "array"),
          nullable: flagBool(flags, "nullable"),
          ...(flagString(flags, "key") !== undefined
            ? { key: Number(flagString(flags, "key")) }
            : {}),
        };
      }
      if (action === "set") {
        return {
          op: "table.column.set",
          table,
          name: rest[3] ?? flagString(flags, "name"),
          ...(flagString(flags, "type") !== undefined
            ? { type: flagString(flags, "type") }
            : {}),
          ...(flagBool(flags, "array") !== undefined
            ? { array: flagBool(flags, "array") }
            : {}),
          ...(flagBool(flags, "nullable") !== undefined
            ? { nullable: flagBool(flags, "nullable") }
            : {}),
          ...(flagString(flags, "key") !== undefined
            ? { key: Number(flagString(flags, "key")) }
            : {}),
        };
      }
      if (action === "remove" || action === "delete") {
        return {
          op: "table.column.remove",
          table,
          name: rest[3] ?? flagString(flags, "name"),
        };
      }
      if (action === "rename") {
        return {
          op: "table.column.rename",
          table,
          old: rest[3] ?? flagString(flags, "old"),
          new: rest[4] ?? flagString(flags, "new"),
        };
      }
      if (action === "move") {
        return {
          op: "table.column.move",
          table,
          name: rest[3] ?? flagString(flags, "name"),
          to: Number(rest[4] ?? flagString(flags, "to")),
        };
      }
    }
    if (sub === "key") {
      const action = rest[1];
      const table = rest[2] ?? flagString(flags, "table");
      if (action === "set-pk") {
        return {
          op: "table.key.setPk",
          table,
          fields: rest[3] ?? flagString(flags, "fields"),
        };
      }
      if (action === "add-sk") {
        return {
          op: "table.key.addSk",
          table,
          fields: rest[3] ?? flagString(flags, "fields"),
          nonUnique: flagBool(flags, "non-unique"),
        };
      }
      if (action === "remove-sk") {
        return {
          op: "table.key.removeSk",
          table,
          index: Number(rest[3] ?? flagString(flags, "index")),
        };
      }
    }
    if (sub === "ref") {
      const action = rest[1];
      const table = rest[2] ?? flagString(flags, "table");
      if (action === "add") {
        return {
          op: "table.ref.add",
          table,
          name: rest[3] ?? flagString(flags, "name"),
          fields: flagString(flags, "fields"),
          targetTable: flagString(flags, "target-table"),
          targetFields: flagString(flags, "target-fields"),
          csharpName: flagString(flags, "csharp-name"),
        };
      }
      if (action === "remove" || action === "delete") {
        return {
          op: "table.ref.remove",
          table,
          name: rest[3] ?? flagString(flags, "name"),
        };
      }
    }
    throw new Error(`未知の table サブコマンドです: ${rest.join(" ")}`);
  }

  if (command === "type") {
    const sub = rest[0];
    if (sub === "create") {
      const membersRaw = flagString(flags, "members");
      const fieldsRaw = flagString(flags, "fields");
      return {
        op: "type.create",
        typeKind: rest[1] ?? flagString(flags, "kind"),
        name: rest[2] ?? flagString(flags, "name"),
        path: flagString(flags, "path") ?? flagString(flags, "file"),
        underlying: flagString(flags, "underlying"),
        ...(membersRaw
          ? {
              members: membersRaw.trim().startsWith("[")
                ? parseStructured(membersRaw)
                : splitCsv(membersRaw),
            }
          : {}),
        ...(fieldsRaw
          ? {
              fields: fieldsRaw.trim().startsWith("[")
                ? parseStructured(fieldsRaw)
                : splitCsv(fieldsRaw),
            }
          : {}),
      };
    }
    if (sub === "delete" || sub === "remove") {
      return { op: "type.delete", name: rest[1] ?? flagString(flags, "name") };
    }
    if (sub === "set-underlying") {
      return {
        op: "type.setUnderlying",
        name: rest[1] ?? flagString(flags, "name"),
        underlying: rest[2] ?? flagString(flags, "underlying"),
      };
    }
    if (sub === "member") {
      const action = rest[1];
      const typeName = rest[2] ?? flagString(flags, "type");
      if (action === "add") {
        return {
          op: "type.member.add",
          type: typeName,
          member: rest[3] ?? flagString(flags, "name"),
          ...(flagString(flags, "value") !== undefined
            ? { value: flagString(flags, "value") }
            : {}),
        };
      }
      if (action === "set") {
        return {
          op: "type.member.set",
          type: typeName,
          member: rest[3] ?? flagString(flags, "name"),
          value: rest[4] ?? flagString(flags, "value"),
        };
      }
      if (action === "remove" || action === "delete") {
        return {
          op: "type.member.remove",
          type: typeName,
          member: rest[3] ?? flagString(flags, "name"),
        };
      }
      if (action === "rename") {
        return {
          op: "type.member.rename",
          type: typeName,
          old: rest[3] ?? flagString(flags, "old"),
          new: rest[4] ?? flagString(flags, "new"),
        };
      }
      if (action === "move") {
        return {
          op: "type.member.move",
          type: typeName,
          member: rest[3] ?? flagString(flags, "name"),
          to: Number(rest[4] ?? flagString(flags, "to")),
        };
      }
    }
    if (sub === "field") {
      const action = rest[1];
      const typeName = rest[2] ?? flagString(flags, "type");
      if (action === "add") {
        return {
          op: "type.field.add",
          type: typeName,
          name: rest[3] ?? flagString(flags, "name"),
          fieldType: rest[4] ?? flagString(flags, "field-type"),
          array: flagBool(flags, "array"),
          nullable: flagBool(flags, "nullable"),
          ...(flagString(flags, "key") !== undefined
            ? { key: Number(flagString(flags, "key")) }
            : {}),
        };
      }
      if (action === "set") {
        return {
          op: "type.field.set",
          type: typeName,
          name: rest[3] ?? flagString(flags, "name"),
          ...(flagString(flags, "field-type") !== undefined
            ? { fieldType: flagString(flags, "field-type") }
            : flagString(flags, "type-name") !== undefined
              ? { fieldType: flagString(flags, "type-name") }
              : {}),
          ...(flagBool(flags, "array") !== undefined
            ? { array: flagBool(flags, "array") }
            : {}),
          ...(flagBool(flags, "nullable") !== undefined
            ? { nullable: flagBool(flags, "nullable") }
            : {}),
          ...(flagString(flags, "key") !== undefined
            ? { key: Number(flagString(flags, "key")) }
            : {}),
        };
      }
      if (action === "remove" || action === "delete") {
        return {
          op: "type.field.remove",
          type: typeName,
          name: rest[3] ?? flagString(flags, "name"),
        };
      }
      if (action === "rename") {
        return {
          op: "type.field.rename",
          type: typeName,
          old: rest[3] ?? flagString(flags, "old"),
          new: rest[4] ?? flagString(flags, "new"),
        };
      }
      if (action === "move") {
        return {
          op: "type.field.move",
          type: typeName,
          name: rest[3] ?? flagString(flags, "name"),
          to: Number(rest[4] ?? flagString(flags, "to")),
        };
      }
    }
    throw new Error(`未知の type サブコマンドです: ${rest.join(" ")}`);
  }

  if (command === "rename") {
    const kind = rest[0];
    if (kind === "table")
      return { op: "rename.table", old: rest[1], new: rest[2] };
    if (kind === "type")
      return { op: "rename.type", old: rest[1], new: rest[2] };
    if (kind === "field" || kind === "column")
      return {
        op: "rename.field",
        target: rest[1],
        old: rest[2],
        new: rest[3],
      };
    if (kind === "member")
      return {
        op: "rename.member",
        target: rest[1],
        old: rest[2],
        new: rest[3],
      };
    throw new Error(
      `rename の種類は table / type / field / member を指定してください: ${kind}`,
    );
  }

  if (command === "source") {
    const sub = rest[0];
    if (sub === "move" || sub === "rename")
      return { op: "source.move", from: rest[1], to: rest[2] };
    if (sub === "delete" || sub === "remove")
      return { op: "source.delete", path: rest[1] };
    throw new Error(`未知の source サブコマンドです: ${rest.join(" ")}`);
  }

  if (command === "fix" || command === "fmt") {
    return {
      op: "fix",
      table: rest[0] ?? flagString(flags, "table"),
    };
  }

  if (command === "record") {
    const sub = rest[0];
    const tableName = rest[1] ?? flagString(flags, "table") ?? "";
    const table = findTable(snapshot, tableName);

    if (sub === "add") {
      const assignmentTokens = rest.slice(2);
      const assignments = parseAssignments(
        snapshot,
        table.fields,
        assignmentTokens,
      );
      const payloadRaw = flagString(flags, "data") ?? flagString(flags, "record");
      let values: Data = undefined;
      let records: Data[] | undefined = undefined;
      if (payloadRaw) {
        const parsed = parseStructured(payloadRaw);
        if (Array.isArray(parsed)) records = parsed;
        else values = parsed;
      }
      return {
        op: "record.add",
        table: tableName,
        source: flagString(flags, "source") ?? flagString(flags, "file"),
        ...(records ? { records } : {}),
        ...(values ? { values } : {}),
        ...(assignments.length ? { assignments } : {}),
        ...(flagString(flags, "tags")
          ? { tags: splitCsv(flagString(flags, "tags")) }
          : {}),
        autoKey: flagBool(flags, "auto-key"),
      };
    }

    if (sub === "set") {
      let pk = flagString(flags, "pk");
      let assignStart = 2;
      if (
        pk === undefined &&
        !flagStrings(flags, "where").length &&
        !flagBool(flags, "all") &&
        rest[2] &&
        !rest[2].includes("=")
      ) {
        pk = rest[2];
        assignStart = 3;
      }
      const assignmentTokens = rest.slice(assignStart);
      const assignments = parseAssignments(
        snapshot,
        table.fields,
        assignmentTokens,
      );
      const patchRaw = flagString(flags, "patch") ?? flagString(flags, "data");
      const setObj = patchRaw ? parseStructured(patchRaw) : undefined;
      return {
        op: "record.set",
        table: tableName,
        ...(pk !== undefined ? { pk } : {}),
        ...(flagStrings(flags, "where").length
          ? { where: flagStrings(flags, "where") }
          : {}),
        all: flagBool(flags, "all"),
        source: flagString(flags, "source") ?? flagString(flags, "file"),
        ...(setObj ? { set: setObj } : {}),
        ...(assignments.length ? { assignments } : {}),
        ...(flagString(flags, "tags") !== undefined
          ? { tags: splitCsv(flagString(flags, "tags")) }
          : {}),
        ...(flagStrings(flags, "add-tag").length
          ? { addTags: flagStrings(flags, "add-tag").flatMap(splitCsv) }
          : {}),
        ...(flagStrings(flags, "remove-tag").length
          ? { removeTags: flagStrings(flags, "remove-tag").flatMap(splitCsv) }
          : {}),
      };
    }

    if (sub === "delete" || sub === "remove") {
      let pk = flagString(flags, "pk");
      if (
        pk === undefined &&
        !flagStrings(flags, "where").length &&
        !flagBool(flags, "all") &&
        rest[2]
      ) {
        pk = rest[2];
      }
      return {
        op: "record.delete",
        table: tableName,
        ...(pk !== undefined ? { pk } : {}),
        ...(flagStrings(flags, "where").length
          ? { where: flagStrings(flags, "where") }
          : {}),
        all: flagBool(flags, "all"),
        source: flagString(flags, "source") ?? flagString(flags, "file"),
      };
    }
  }

  throw new Error(`未知のコマンドです: ${command}`);
}

export async function runCli(
  argv: string[],
  options: CliOptions = {},
): Promise<CliResult> {
  const cwd = options.cwd ?? process.cwd();
  const parsed = parseArgs(argv);
  const wantJson = !!flagBool(parsed.flags, "json");
  const dryRun = !!flagBool(parsed.flags, "dry-run");
  const strict = !!flagBool(parsed.flags, "strict");
  const command = parsed.positionals[0];
  const rest = parsed.positionals.slice(1);

  const ok = (payload: Record<string, any>, textSummary: string): CliResult => ({
    exitCode: 0,
    stdout: wantJson ? `${stringifyJson({ ok: true, ...payload })}\n` : `${textSummary}\n`,
    stderr: "",
    data: { ok: true, ...payload },
  });

  const fail = (error: unknown, extra: Record<string, any> = {}): CliResult => {
    const message = error instanceof Error ? error.message : String(error);
    const problems = (error as any)?.problems;
    const body = {
      ok: false,
      error: message,
      ...(problems ? { problems } : {}),
      ...extra,
    };
    return {
      exitCode: 1,
      stdout: wantJson ? `${stringifyJson(body)}\n` : "",
      stderr: wantJson ? "" : `${message}\n`,
      data: body,
    };
  };

  try {
    if (!command || command === "help" || flagBool(parsed.flags, "help")) {
      return ok({ help: HELP_TEXT }, HELP_TEXT.trimEnd());
    }

    if (command === "init") {
      const targetDir = path.resolve(cwd, rest[0] ?? ".");
      const projectName = (
        flagString(parsed.flags, "name") ?? path.basename(targetDir)
      ).trim();
      if (!projectName)
        throw new Error("プロジェクト名を --name で指定してください。");
      const tomlPath = path.join(targetDir, projectFileName);
      if (await fs.lstat(tomlPath).catch(() => null))
        throw new Error(`すでに ${projectFileName} が存在します: ${tomlPath}`);
      await fs.mkdir(path.join(targetDir, "sources"), { recursive: true });
      await fs.writeFile(tomlPath, writeConfig(defaultConfig(projectName)));
      return ok(
        {
          command: "init",
          root: targetDir,
          projectFile: tomlPath,
          name: projectName,
        },
        `プロジェクト「${projectName}」を作成しました: ${tomlPath}`,
      );
    }

    const projectFile = await resolveProjectFile(
      flagString(parsed.flags, "project"),
      cwd,
    );
    const { workspace, project } = await loadWorkspace(projectFile);
    const session = new Session();
    session.load(project);

    if (command === "inspect") {
      const target = rest[0] ?? flagString(parsed.flags, "target");
      const info = inspectProject(session.state.snapshot, target);
      if (target) {
        return ok(
          { command: "inspect", root: workspace.root, target, detail: info },
          stringifyJson(info),
        );
      }
      const summaryLines = [
        `Project: ${project.snapshot.config.name} (${workspace.root})`,
        `Tables (${(info as any).tables.length}): ${(info as any).tables.map((t: any) => `${t.name}[${t.recordCount}]`).join(", ") || "なし"}`,
        `Types (${(info as any).types.length}): ${(info as any).types.map((t: any) => `${t.name}(${t.kind})`).join(", ") || "なし"}`,
        `Problems: ${(info as any).problems.total} (syntaxErrors: ${(info as any).problems.syntaxErrorCount})`,
      ];
      return ok(
        { command: "inspect", root: workspace.root, ...info },
        summaryLines.join("\n"),
      );
    }

    if (command === "validate") {
      const profile = flagString(parsed.flags, "profile") ?? "";
      const problems = formatProblems(session.state.snapshot, profile);
      if (problems.length > 0) {
        const lines = problems.map(
          (p) => `[${p.code}] ${p.file} (${p.path.join(".")}): ${p.message}`,
        );
        return {
          exitCode: 1,
          stdout: wantJson
            ? `${stringifyJson({
                ok: false,
                command: "validate",
                profile: profile || null,
                problemCount: problems.length,
                problems,
              })}\n`
            : "",
          stderr: wantJson
            ? ""
            : `検証エラーが ${problems.length} 件見つかりました:\n${lines.join("\n")}\n`,
          data: {
            ok: false,
            command: "validate",
            profile: profile || null,
            problemCount: problems.length,
            problems,
          },
        };
      }
      return ok(
        {
          command: "validate",
          profile: profile || null,
          problemCount: 0,
          problems: [],
        },
        `検証OK（0件のエラー${profile ? `, profile: ${profile}` : ""}）`,
      );
    }

    if (command === "config" && (rest[0] ?? "show") === "show") {
      return ok(
        { command: "config.show", config: session.state.snapshot.config },
        stringifyJson(session.state.snapshot.config),
      );
    }

    if (
      command === "record" &&
      (rest[0] === "list" || rest[0] === "get")
    ) {
      const sub = rest[0];
      const tableName = rest[1] ?? flagString(parsed.flags, "table") ?? "";
      const pk =
        sub === "get"
          ? rest[2] ?? flagString(parsed.flags, "pk")
          : flagString(parsed.flags, "pk");
      if (sub === "get" && pk === undefined)
        throw new Error("取得するレコードの主キーを指定してください。");
      const result = queryRecords(session.state.snapshot, tableName, {
        pk,
        where: flagStrings(parsed.flags, "where"),
        source: flagString(parsed.flags, "source") ?? flagString(parsed.flags, "file"),
        profile: flagString(parsed.flags, "profile"),
        limit:
          flagString(parsed.flags, "limit") !== undefined
            ? Number(flagString(parsed.flags, "limit"))
            : undefined,
        offset:
          flagString(parsed.flags, "offset") !== undefined
            ? Number(flagString(parsed.flags, "offset"))
            : undefined,
      });
      if (sub === "get" && result.records.length === 0)
        throw new Error(`主キーに一致するレコードが見つかりません: ${String(pk)}`);
      const payload =
        sub === "get"
          ? {
              command: "record.get",
              table: tableName,
              source: result.records[0].source,
              record: result.records[0].record,
            }
          : {
              command: "record.list",
              ...result,
            };
      return ok(payload, stringifyJson(sub === "get" ? result.records[0].record : result.records));
    }

    if (command === "tx" || command === "apply") {
      const fileOpt = flagString(parsed.flags, "file");
      const opsOpt = flagString(parsed.flags, "ops");
      let rawInput: string;
      if (fileOpt) {
        rawInput = await fs.readFile(path.resolve(cwd, fileOpt), "utf8");
      } else if (opsOpt) {
        rawInput = opsOpt;
      } else if (options.stdin !== undefined) {
        rawInput = options.stdin;
      } else {
        const chunks: Buffer[] = [];
        for await (const chunk of process.stdin) chunks.push(Buffer.from(chunk));
        rawInput = Buffer.concat(chunks).toString("utf8");
      }
      if (!rawInput.trim())
        throw new Error("tx に渡す操作リスト（JSONまたはYAML）が空です。");
      const parsedPayload = parseStructured(rawInput);
      const ops: Record<string, any>[] = Array.isArray(parsedPayload)
        ? parsedPayload
        : isObject(parsedPayload) && Array.isArray(parsedPayload.operations)
          ? parsedPayload.operations
          : (() => {
              throw new Error(
                "tx の入力は操作オブジェクトの配列 `[...]` または `{ operations: [...] }` で指定してください。",
              );
            })();
      const results = ops.map((op) => applyOperation(session, op));
      const committed = await commitSession(workspace, session, {
        dryRun,
        strict,
      });
      return ok(
        {
          command: "tx",
          operationCount: results.length,
          results,
          ...committed,
        },
        `トランザクション完了（${results.length}操作, 変更ファイル: ${committed.changedFiles.length}件, 現在の診断エラー: ${committed.problemCount}件）`,
      );
    }

    if (command === "build") {
      const profile = flagString(parsed.flags, "profile") ?? "";
      const result = await buildWorkspace(
        workspace,
        session.state.snapshot,
        profile,
        1,
      );
      const latestFile = await safePath(
        workspace.root,
        ".masterdata/latest-build.json",
      );
      await fs.mkdir(path.dirname(latestFile), { recursive: true });
      await fs.writeFile(
        latestFile,
        JSON.stringify(
          {
            workspaceDigest: computeWorkspaceDigest(project),
            builtAt: new Date().toISOString(),
            result,
          },
          null,
          2,
        ),
      );
      return ok(
        { command: "build", result },
        `Build成功（tables: ${result.tables}, records: ${result.records}, output: ${result.directory}）`,
      );
    }

    if (command === "publish") {
      const shouldBuild = !!flagBool(parsed.flags, "build");
      const profile = flagString(parsed.flags, "profile") ?? "";
      const latestFile = await safePath(
        workspace.root,
        ".masterdata/latest-build.json",
      );
      const currentDigest = computeWorkspaceDigest(project);
      let buildResult: BuildResult;

      if (shouldBuild) {
        buildResult = await buildWorkspace(
          workspace,
          session.state.snapshot,
          profile,
          1,
        );
        await fs.mkdir(path.dirname(latestFile), { recursive: true });
        await fs.writeFile(
          latestFile,
          JSON.stringify(
            {
              workspaceDigest: currentDigest,
              builtAt: new Date().toISOString(),
              result: buildResult,
            },
            null,
            2,
          ),
        );
      } else {
        const latestText = await optionalRead(latestFile);
        if (!latestText)
          throw new Error("先に `masterdata build` を実行してください。");
        const latest = JSON.parse(latestText) as {
          workspaceDigest: string;
          result: BuildResult;
        };
        if (latest.workspaceDigest !== currentDigest)
          throw new Error(
            "最後のBuild以降にソースまたは設定が変更されています。先に `masterdata build` を再実行するか `masterdata publish --build` を使用してください。",
          );
        buildResult = latest.result;
      }

      const targets = {
        csharp:
          flagString(parsed.flags, "csharp") ??
          session.state.snapshot.config.publish.csharp,
        binary:
          flagString(parsed.flags, "binary") ??
          session.state.snapshot.config.publish.binary,
      };
      const message = await publishBuild(buildResult, workspace.root, targets);
      return ok(
        {
          command: "publish",
          targets,
          buildDirectory: buildResult.directory,
          message,
        },
        message,
      );
    }

    if (
      ["config", "table", "type", "rename", "source", "fix", "fmt", "record"].includes(
        command,
      )
    ) {
      const op = buildCliOperation(session, command, rest, parsed.flags);
      const result = applyOperation(session, op);
      const committed = await commitSession(workspace, session, {
        dryRun,
        strict,
      });
      return ok(
        {
          command: op.op,
          result,
          ...committed,
        },
        `${op.op} 完了（変更ファイル: ${committed.changedFiles.length}件, 現在の診断エラー: ${committed.problemCount}件）`,
      );
    }

    throw new Error(`未知のコマンドです: ${command}`);
  } catch (error) {
    return fail(error);
  }
}
