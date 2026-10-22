use crate::build::{build_workspace, publish_build};
use crate::config::{write_config, PROJECT_FILE_NAME};
use crate::files::{
    compute_workspace_digest, load_workspace, optional_read, safe_path, save_workspace, Workspace,
};
use crate::model::{
    default_config, BuildResult, Data, PublishConfig, SaveRequest,
};
use crate::operations::{
    apply_operation, find_table, format_problems, inspect_project, parse_assignments,
    query_records, split_csv, EditSession, PathStep, QueryOptions,
};
use crate::yaml::parse_structured_data;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct CliOptions {
    pub cwd: Option<PathBuf>,
    pub stdin: Option<String>,
}

pub struct CliResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub enum FlagVal {
    Str(String),
    Bool(bool),
}

pub struct ParsedArgs {
    pub positionals: Vec<String>,
    pub flags: IndexMap<String, Vec<FlagVal>>,
}

const BOOLEAN_FLAGS: &[&str] = &[
    "json",
    "help",
    "h",
    "dry-run",
    "strict",
    "array",
    "nullable",
    "no-array",
    "no-nullable",
    "non-unique",
    "all",
    "auto-key",
    "build",
];

pub const HELP_TEXT: &str = "MasterData CLI — 構造を理解するマスターデータ管理・編集・ビルドツール

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
";

pub fn parse_args(argv: &[String]) -> ParsedArgs {
    let mut positionals = Vec::new();
    let mut flags: IndexMap<String, Vec<FlagVal>> = IndexMap::new();
    let bool_set: HashSet<&str> = BOOLEAN_FLAGS.iter().copied().collect();

    let mut i = 0;
    while i < argv.len() {
        let token = &argv[i];
        if token == "--" {
            positionals.extend_from_slice(&argv[i + 1..]);
            break;
        }
        if token == "-h" {
            flags
                .entry("help".to_string())
                .or_default()
                .push(FlagVal::Bool(true));
            i += 1;
            continue;
        }
        if let Some(stripped) = token.strip_prefix("--") {
            if let Some(eq) = stripped.find('=') {
                let key = stripped[..eq].to_string();
                let val = stripped[eq + 1..].to_string();
                flags.entry(key).or_default().push(FlagVal::Str(val));
                i += 1;
                continue;
            }
            let key = stripped.to_string();
            if bool_set.contains(key.as_str()) {
                if let Some(next) = argv.get(i + 1) {
                    if next == "true" || next == "false" {
                        flags
                            .entry(key)
                            .or_default()
                            .push(FlagVal::Bool(next == "true"));
                        i += 2;
                        continue;
                    }
                }
                flags.entry(key).or_default().push(FlagVal::Bool(true));
                i += 1;
                continue;
            }
            if let Some(next) = argv.get(i + 1) {
                if !next.starts_with("--") {
                    flags
                        .entry(key)
                        .or_default()
                        .push(FlagVal::Str(next.clone()));
                    i += 2;
                    continue;
                }
            }
            flags.entry(key).or_default().push(FlagVal::Bool(true));
            i += 1;
            continue;
        }
        positionals.push(token.clone());
        i += 1;
    }

    ParsedArgs { positionals, flags }
}

pub fn flag_string(flags: &IndexMap<String, Vec<FlagVal>>, name: &str) -> Option<String> {
    flags.get(name)?.last().and_then(|v| match v {
        FlagVal::Str(s) => Some(s.clone()),
        FlagVal::Bool(_) => None,
    })
}

pub fn flag_strings(flags: &IndexMap<String, Vec<FlagVal>>, name: &str) -> Vec<String> {
    flags
        .get(name)
        .map(|list| {
            list.iter()
                .filter_map(|v| match v {
                    FlagVal::Str(s) => Some(s.clone()),
                    FlagVal::Bool(_) => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn flag_bool(flags: &IndexMap<String, Vec<FlagVal>>, name: &str) -> Option<bool> {
    if flags.contains_key(&format!("no-{name}")) {
        return Some(false);
    }
    flags.get(name)?.last().and_then(|v| match v {
        FlagVal::Bool(b) => Some(*b),
        FlagVal::Str(s) if s == "true" => Some(true),
        FlagVal::Str(s) if s == "false" => Some(false),
        _ => None,
    })
}

pub fn stringify_json(value: &Data) -> String {
    serde_json::to_string_pretty(&value.to_json_value()).unwrap_or_else(|_| "{}".to_string())
}

pub fn resolve_project_file(project_opt: Option<&str>, cwd: &Path) -> Result<PathBuf, String> {
    if let Some(opt) = project_opt {
        let resolved = cwd.join(opt);
        let meta = fs::symlink_metadata(&resolved)
            .map_err(|_| format!("指定されたプロジェクトが見つかりません: {opt}"))?;
        if meta.is_file() {
            if resolved.file_name().and_then(|n| n.to_str()) != Some(PROJECT_FILE_NAME) {
                return Err(format!(
                    "{PROJECT_FILE_NAME} ファイルまたはプロジェクトフォルダを指定してください: {opt}"
                ));
            }
            return Ok(resolved);
        }
        if meta.is_dir() {
            let candidate = resolved.join(PROJECT_FILE_NAME);
            if fs::symlink_metadata(&candidate)
                .map(|m| m.is_file())
                .unwrap_or(false)
            {
                return Ok(candidate);
            }
            return Err(format!(
                "指定フォルダ内に {PROJECT_FILE_NAME} が見つかりません: {opt}"
            ));
        }
        return Err(format!("不正なプロジェクト指定です: {opt}"));
    }

    let mut current = cwd.to_path_buf();
    loop {
        let candidate = current.join(PROJECT_FILE_NAME);
        if fs::symlink_metadata(&candidate)
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            return Ok(candidate);
        }
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent.to_path_buf();
    }
    Err(format!(
        "{PROJECT_FILE_NAME} が見つかりません。`masterdata init` で作成するか `--project` を指定してください。"
    ))
}

struct CommitError {
    message: String,
    problems: Option<Vec<Data>>,
}

impl From<String> for CommitError {
    fn from(message: String) -> Self {
        Self {
            message,
            problems: None,
        }
    }
}

fn commit_session(
    workspace: &mut Workspace,
    session: &EditSession,
    dry_run: bool,
    strict: bool,
) -> Result<(IndexMap<String, Data>, usize, usize), CommitError> {
    let problems = format_problems(&session.snapshot, "").map_err(CommitError::from)?;
    if strict && !problems.is_empty() {
        let preview: Vec<String> = problems
            .iter()
            .take(10)
            .map(|p| {
                let code = p.get("code").map(|v| v.text()).unwrap_or_default();
                let file = p.get("file").map(|v| v.text()).unwrap_or_default();
                let msg = p.get("message").map(|v| v.text()).unwrap_or_default();
                format!("  - [{code}] {file}: {msg}")
            })
            .collect();
        return Err(CommitError {
            message: format!(
                "--strict 指定により保存を中止しました（未解決の診断エラー: {}件）\n{}",
                problems.len(),
                preview.join("\n")
            ),
            problems: Some(problems),
        });
    }

    let desired_files = session.files();
    let mut baseline_map = IndexMap::new();
    for f in &workspace.baseline {
        baseline_map.insert(f.path.clone(), f.text.clone());
    }
    let mut desired_map = IndexMap::new();
    for f in &desired_files {
        desired_map.insert(f.path.clone(), f.text.clone());
    }
    let mut changed_sources: Vec<String> = Vec::new();
    for (p, text) in &desired_map {
        if baseline_map.get(p) != Some(text) {
            changed_sources.push(format!("sources/{p}"));
        }
    }
    for p in baseline_map.keys() {
        if !desired_map.contains_key(p) {
            changed_sources.push(format!("sources/{p} (deleted)"));
        }
    }
    let next_config_text = write_config(&session.snapshot.config, Some(&workspace.config_text))
        .map_err(CommitError::from)?;
    if next_config_text != workspace.config_text {
        changed_sources.insert(0, PROJECT_FILE_NAME.to_string());
    }

    if !dry_run {
        save_workspace(
            workspace,
            &SaveRequest {
                token: workspace.token.clone(),
                files: desired_files,
                folders: session.snapshot.folders.clone(),
                config: session.snapshot.config.clone(),
            },
        )
        .map_err(CommitError::from)?;
    }

    let changed_count = changed_sources.len();
    let problem_count = problems.len();
    let mut map = IndexMap::new();
    map.insert("dryRun".to_string(), Data::Bool(dry_run));
    map.insert(
        "changedFiles".to_string(),
        Data::Array(changed_sources.into_iter().map(Data::Str).collect()),
    );
    map.insert(
        "problemCount".to_string(),
        Data::Int(problem_count as i128),
    );
    if !problems.is_empty() {
        map.insert(
            "problems".to_string(),
            Data::Array(problems.into_iter().take(20).collect()),
        );
    }
    Ok((map, changed_count, problem_count))
}

fn path_steps_to_str(steps: &[PathStep]) -> String {
    let mut out = String::new();
    for (i, step) in steps.iter().enumerate() {
        match step {
            PathStep::Key(k) => {
                if i > 0 {
                    out.push('.');
                }
                out.push_str(k);
            }
            PathStep::Index(idx) => {
                out.push_str(&format!("[{idx}]"));
            }
            PathStep::Append => {
                out.push_str("[+]");
            }
        }
    }
    out
}

fn build_cli_operation(
    session: &EditSession,
    command: &str,
    rest: &[String],
    flags: &IndexMap<String, Vec<FlagVal>>,
) -> Result<Data, String> {
    let snapshot = &session.snapshot;
    let mut op = IndexMap::new();

    if command == "config" {
        let sub = rest.first().map(String::as_str).unwrap_or("show");
        if sub == "set" {
            op.insert("op".to_string(), Data::Str("config.set".to_string()));
            if let Some(n) = flag_string(flags, "name") {
                op.insert("name".to_string(), Data::Str(n));
            }
            if let Some(cs) = flag_string(flags, "publish-csharp") {
                op.insert("publishCsharp".to_string(), Data::Str(cs));
            }
            if let Some(bin) = flag_string(flags, "publish-binary") {
                op.insert("publishBinary".to_string(), Data::Str(bin));
            }
            return Ok(Data::Map(op));
        }
        if sub == "profile" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            let name = rest
                .get(2)
                .cloned()
                .or_else(|| flag_string(flags, "name"))
                .unwrap_or_default();
            if action == "set" {
                op.insert("op".to_string(), Data::Str("config.profile.set".to_string()));
                op.insert("name".to_string(), Data::Str(name));
                if let Some(inc) = flag_string(flags, "include") {
                    op.insert(
                        "include".to_string(),
                        Data::Array(split_csv(&inc).into_iter().map(Data::Str).collect()),
                    );
                }
                if let Some(exc) = flag_string(flags, "exclude") {
                    op.insert(
                        "exclude".to_string(),
                        Data::Array(split_csv(&exc).into_iter().map(Data::Str).collect()),
                    );
                }
                return Ok(Data::Map(op));
            }
            if action == "remove" || action == "delete" {
                op.insert(
                    "op".to_string(),
                    Data::Str("config.profile.remove".to_string()),
                );
                op.insert("name".to_string(), Data::Str(name));
                return Ok(Data::Map(op));
            }
        }
        return Err(format!("未知の config サブコマンドです: {}", rest.join(" ")));
    }

    if command == "table" {
        let sub = rest.first().map(String::as_str).unwrap_or("");
        if sub == "create" {
            let name = rest
                .get(1)
                .cloned()
                .or_else(|| flag_string(flags, "name"))
                .unwrap_or_default();
            op.insert("op".to_string(), Data::Str("table.create".to_string()));
            op.insert("name".to_string(), Data::Str(name));
            if let Some(p) = flag_string(flags, "path").or_else(|| flag_string(flags, "file")) {
                op.insert("path".to_string(), Data::Str(p));
            }
            if let Some(pk) = flag_string(flags, "pk") {
                op.insert("pk".to_string(), Data::Str(pk));
            }
            if let Some(cs) = flag_string(flags, "csharp-name") {
                op.insert("csharpName".to_string(), Data::Str(cs));
            }
            if let Some(fraw) = flag_string(flags, "fields") {
                let fields_data = if fraw.trim().starts_with('[') {
                    parse_structured_data(&fraw)?
                } else {
                    Data::Array(split_csv(&fraw).into_iter().map(Data::Str).collect())
                };
                op.insert("fields".to_string(), fields_data);
            }
            return Ok(Data::Map(op));
        }
        if sub == "delete" || sub == "remove" {
            let name = rest
                .get(1)
                .cloned()
                .or_else(|| flag_string(flags, "table"))
                .unwrap_or_default();
            op.insert("op".to_string(), Data::Str("table.delete".to_string()));
            op.insert("name".to_string(), Data::Str(name));
            return Ok(Data::Map(op));
        }
        if sub == "data-file" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            if action == "create" || action == "add" {
                let table = rest
                    .get(2)
                    .cloned()
                    .or_else(|| flag_string(flags, "table"))
                    .unwrap_or_default();
                let path = rest
                    .get(3)
                    .cloned()
                    .or_else(|| flag_string(flags, "path"))
                    .or_else(|| flag_string(flags, "file"))
                    .unwrap_or_default();
                op.insert(
                    "op".to_string(),
                    Data::Str("table.dataFile.create".to_string()),
                );
                op.insert("table".to_string(), Data::Str(table));
                op.insert("path".to_string(), Data::Str(path));
                return Ok(Data::Map(op));
            }
        }
        if sub == "column" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            let table = rest
                .get(2)
                .cloned()
                .or_else(|| flag_string(flags, "table"))
                .unwrap_or_default();
            if action == "add" {
                op.insert("op".to_string(), Data::Str("table.column.add".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                op.insert(
                    "type".to_string(),
                    Data::Str(
                        rest.get(4)
                            .cloned()
                            .or_else(|| flag_string(flags, "type"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(a) = flag_bool(flags, "array") {
                    op.insert("array".to_string(), Data::Bool(a));
                }
                if let Some(n) = flag_bool(flags, "nullable") {
                    op.insert("nullable".to_string(), Data::Bool(n));
                }
                if let Some(k) = flag_string(flags, "key").and_then(|s| s.parse::<i128>().ok()) {
                    op.insert("key".to_string(), Data::Int(k));
                }
                return Ok(Data::Map(op));
            }
            if action == "set" {
                op.insert("op".to_string(), Data::Str("table.column.set".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(t) = flag_string(flags, "type") {
                    op.insert("type".to_string(), Data::Str(t));
                }
                if let Some(a) = flag_bool(flags, "array") {
                    op.insert("array".to_string(), Data::Bool(a));
                }
                if let Some(n) = flag_bool(flags, "nullable") {
                    op.insert("nullable".to_string(), Data::Bool(n));
                }
                if let Some(k) = flag_string(flags, "key").and_then(|s| s.parse::<i128>().ok()) {
                    op.insert("key".to_string(), Data::Int(k));
                }
                return Ok(Data::Map(op));
            }
            if action == "remove" || action == "delete" {
                op.insert(
                    "op".to_string(),
                    Data::Str("table.column.remove".to_string()),
                );
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "rename" {
                op.insert(
                    "op".to_string(),
                    Data::Str("table.column.rename".to_string()),
                );
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "old".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "old"))
                            .unwrap_or_default(),
                    ),
                );
                op.insert(
                    "new".to_string(),
                    Data::Str(
                        rest.get(4)
                            .cloned()
                            .or_else(|| flag_string(flags, "new"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "move" {
                op.insert("op".to_string(), Data::Str("table.column.move".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                let to_val = rest
                    .get(4)
                    .cloned()
                    .or_else(|| flag_string(flags, "to"))
                    .and_then(|s| s.parse::<i128>().ok())
                    .unwrap_or(-1);
                op.insert("to".to_string(), Data::Int(to_val));
                return Ok(Data::Map(op));
            }
        }
        if sub == "key" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            let table = rest
                .get(2)
                .cloned()
                .or_else(|| flag_string(flags, "table"))
                .unwrap_or_default();
            if action == "set-pk" {
                op.insert("op".to_string(), Data::Str("table.key.setPk".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "fields".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "fields"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "add-sk" {
                op.insert("op".to_string(), Data::Str("table.key.addSk".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "fields".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "fields"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(nu) = flag_bool(flags, "non-unique") {
                    op.insert("nonUnique".to_string(), Data::Bool(nu));
                }
                return Ok(Data::Map(op));
            }
            if action == "remove-sk" {
                op.insert(
                    "op".to_string(),
                    Data::Str("table.key.removeSk".to_string()),
                );
                op.insert("table".to_string(), Data::Str(table));
                let idx = rest
                    .get(3)
                    .cloned()
                    .or_else(|| flag_string(flags, "index"))
                    .and_then(|s| s.parse::<i128>().ok())
                    .unwrap_or(-1);
                op.insert("index".to_string(), Data::Int(idx));
                return Ok(Data::Map(op));
            }
        }
        if sub == "ref" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            let table = rest
                .get(2)
                .cloned()
                .or_else(|| flag_string(flags, "table"))
                .unwrap_or_default();
            if action == "add" {
                op.insert("op".to_string(), Data::Str("table.ref.add".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(f) = flag_string(flags, "fields") {
                    op.insert("fields".to_string(), Data::Str(f));
                }
                if let Some(tt) = flag_string(flags, "target-table") {
                    op.insert("targetTable".to_string(), Data::Str(tt));
                }
                if let Some(tf) = flag_string(flags, "target-fields") {
                    op.insert("targetFields".to_string(), Data::Str(tf));
                }
                if let Some(cs) = flag_string(flags, "csharp-name") {
                    op.insert("csharpName".to_string(), Data::Str(cs));
                }
                return Ok(Data::Map(op));
            }
            if action == "remove" || action == "delete" {
                op.insert("op".to_string(), Data::Str("table.ref.remove".to_string()));
                op.insert("table".to_string(), Data::Str(table));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
        }
        return Err(format!("未知の table サブコマンドです: {}", rest.join(" ")));
    }

    if command == "type" {
        let sub = rest.first().map(String::as_str).unwrap_or("");
        if sub == "create" {
            op.insert("op".to_string(), Data::Str("type.create".to_string()));
            op.insert(
                "typeKind".to_string(),
                Data::Str(
                    rest.get(1)
                        .cloned()
                        .or_else(|| flag_string(flags, "kind"))
                        .unwrap_or_default(),
                ),
            );
            op.insert(
                "name".to_string(),
                Data::Str(
                    rest.get(2)
                        .cloned()
                        .or_else(|| flag_string(flags, "name"))
                        .unwrap_or_default(),
                ),
            );
            if let Some(p) = flag_string(flags, "path").or_else(|| flag_string(flags, "file")) {
                op.insert("path".to_string(), Data::Str(p));
            }
            if let Some(u) = flag_string(flags, "underlying") {
                op.insert("underlying".to_string(), Data::Str(u));
            }
            if let Some(mraw) = flag_string(flags, "members") {
                let mdata = if mraw.trim().starts_with('[') {
                    parse_structured_data(&mraw)?
                } else {
                    Data::Array(split_csv(&mraw).into_iter().map(Data::Str).collect())
                };
                op.insert("members".to_string(), mdata);
            }
            if let Some(fraw) = flag_string(flags, "fields") {
                let fdata = if fraw.trim().starts_with('[') {
                    parse_structured_data(&fraw)?
                } else {
                    Data::Array(split_csv(&fraw).into_iter().map(Data::Str).collect())
                };
                op.insert("fields".to_string(), fdata);
            }
            return Ok(Data::Map(op));
        }
        if sub == "delete" || sub == "remove" {
            op.insert("op".to_string(), Data::Str("type.delete".to_string()));
            op.insert(
                "name".to_string(),
                Data::Str(
                    rest.get(1)
                        .cloned()
                        .or_else(|| flag_string(flags, "name"))
                        .unwrap_or_default(),
                ),
            );
            return Ok(Data::Map(op));
        }
        if sub == "set-underlying" {
            op.insert(
                "op".to_string(),
                Data::Str("type.setUnderlying".to_string()),
            );
            op.insert(
                "name".to_string(),
                Data::Str(
                    rest.get(1)
                        .cloned()
                        .or_else(|| flag_string(flags, "name"))
                        .unwrap_or_default(),
                ),
            );
            op.insert(
                "underlying".to_string(),
                Data::Str(
                    rest.get(2)
                        .cloned()
                        .or_else(|| flag_string(flags, "underlying"))
                        .unwrap_or_default(),
                ),
            );
            return Ok(Data::Map(op));
        }
        if sub == "member" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            let type_name = rest
                .get(2)
                .cloned()
                .or_else(|| flag_string(flags, "type"))
                .unwrap_or_default();
            if action == "add" {
                op.insert("op".to_string(), Data::Str("type.member.add".to_string()));
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "member".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(v) = flag_string(flags, "value") {
                    op.insert("value".to_string(), Data::Str(v));
                }
                return Ok(Data::Map(op));
            }
            if action == "set" {
                op.insert("op".to_string(), Data::Str("type.member.set".to_string()));
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "member".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(v) = rest.get(4).cloned().or_else(|| flag_string(flags, "value")) {
                    op.insert("value".to_string(), Data::Str(v));
                }
                return Ok(Data::Map(op));
            }
            if action == "remove" || action == "delete" {
                op.insert(
                    "op".to_string(),
                    Data::Str("type.member.remove".to_string()),
                );
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "member".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "rename" {
                op.insert(
                    "op".to_string(),
                    Data::Str("type.member.rename".to_string()),
                );
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "old".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "old"))
                            .unwrap_or_default(),
                    ),
                );
                op.insert(
                    "new".to_string(),
                    Data::Str(
                        rest.get(4)
                            .cloned()
                            .or_else(|| flag_string(flags, "new"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "move" {
                op.insert("op".to_string(), Data::Str("type.member.move".to_string()));
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "member".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                let to_val = rest
                    .get(4)
                    .cloned()
                    .or_else(|| flag_string(flags, "to"))
                    .and_then(|s| s.parse::<i128>().ok())
                    .unwrap_or(-1);
                op.insert("to".to_string(), Data::Int(to_val));
                return Ok(Data::Map(op));
            }
        }
        if sub == "field" {
            let action = rest.get(1).map(String::as_str).unwrap_or("");
            let type_name = rest
                .get(2)
                .cloned()
                .or_else(|| flag_string(flags, "type"))
                .unwrap_or_default();
            if action == "add" {
                op.insert("op".to_string(), Data::Str("type.field.add".to_string()));
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                op.insert(
                    "fieldType".to_string(),
                    Data::Str(
                        rest.get(4)
                            .cloned()
                            .or_else(|| flag_string(flags, "field-type"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(a) = flag_bool(flags, "array") {
                    op.insert("array".to_string(), Data::Bool(a));
                }
                if let Some(n) = flag_bool(flags, "nullable") {
                    op.insert("nullable".to_string(), Data::Bool(n));
                }
                if let Some(k) = flag_string(flags, "key").and_then(|s| s.parse::<i128>().ok()) {
                    op.insert("key".to_string(), Data::Int(k));
                }
                return Ok(Data::Map(op));
            }
            if action == "set" {
                op.insert("op".to_string(), Data::Str("type.field.set".to_string()));
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                if let Some(ft) =
                    flag_string(flags, "field-type").or_else(|| flag_string(flags, "type-name"))
                {
                    op.insert("fieldType".to_string(), Data::Str(ft));
                }
                if let Some(a) = flag_bool(flags, "array") {
                    op.insert("array".to_string(), Data::Bool(a));
                }
                if let Some(n) = flag_bool(flags, "nullable") {
                    op.insert("nullable".to_string(), Data::Bool(n));
                }
                if let Some(k) = flag_string(flags, "key").and_then(|s| s.parse::<i128>().ok()) {
                    op.insert("key".to_string(), Data::Int(k));
                }
                return Ok(Data::Map(op));
            }
            if action == "remove" || action == "delete" {
                op.insert(
                    "op".to_string(),
                    Data::Str("type.field.remove".to_string()),
                );
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "rename" {
                op.insert(
                    "op".to_string(),
                    Data::Str("type.field.rename".to_string()),
                );
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "old".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "old"))
                            .unwrap_or_default(),
                    ),
                );
                op.insert(
                    "new".to_string(),
                    Data::Str(
                        rest.get(4)
                            .cloned()
                            .or_else(|| flag_string(flags, "new"))
                            .unwrap_or_default(),
                    ),
                );
                return Ok(Data::Map(op));
            }
            if action == "move" {
                op.insert("op".to_string(), Data::Str("type.field.move".to_string()));
                op.insert("type".to_string(), Data::Str(type_name));
                op.insert(
                    "name".to_string(),
                    Data::Str(
                        rest.get(3)
                            .cloned()
                            .or_else(|| flag_string(flags, "name"))
                            .unwrap_or_default(),
                    ),
                );
                let to_val = rest
                    .get(4)
                    .cloned()
                    .or_else(|| flag_string(flags, "to"))
                    .and_then(|s| s.parse::<i128>().ok())
                    .unwrap_or(-1);
                op.insert("to".to_string(), Data::Int(to_val));
                return Ok(Data::Map(op));
            }
        }
        return Err(format!("未知の type サブコマンドです: {}", rest.join(" ")));
    }

    if command == "rename" {
        let kind = rest.first().map(String::as_str).unwrap_or("");
        if kind == "table" {
            op.insert("op".to_string(), Data::Str("rename.table".to_string()));
            op.insert(
                "old".to_string(),
                Data::Str(rest.get(1).cloned().unwrap_or_default()),
            );
            op.insert(
                "new".to_string(),
                Data::Str(rest.get(2).cloned().unwrap_or_default()),
            );
            return Ok(Data::Map(op));
        }
        if kind == "type" {
            op.insert("op".to_string(), Data::Str("rename.type".to_string()));
            op.insert(
                "old".to_string(),
                Data::Str(rest.get(1).cloned().unwrap_or_default()),
            );
            op.insert(
                "new".to_string(),
                Data::Str(rest.get(2).cloned().unwrap_or_default()),
            );
            return Ok(Data::Map(op));
        }
        if kind == "field" || kind == "column" {
            op.insert("op".to_string(), Data::Str("rename.field".to_string()));
            op.insert(
                "target".to_string(),
                Data::Str(rest.get(1).cloned().unwrap_or_default()),
            );
            op.insert(
                "old".to_string(),
                Data::Str(rest.get(2).cloned().unwrap_or_default()),
            );
            op.insert(
                "new".to_string(),
                Data::Str(rest.get(3).cloned().unwrap_or_default()),
            );
            return Ok(Data::Map(op));
        }
        if kind == "member" {
            op.insert("op".to_string(), Data::Str("rename.member".to_string()));
            op.insert(
                "target".to_string(),
                Data::Str(rest.get(1).cloned().unwrap_or_default()),
            );
            op.insert(
                "old".to_string(),
                Data::Str(rest.get(2).cloned().unwrap_or_default()),
            );
            op.insert(
                "new".to_string(),
                Data::Str(rest.get(3).cloned().unwrap_or_default()),
            );
            return Ok(Data::Map(op));
        }
        return Err(format!(
            "rename の種類は table / type / field / member を指定してください: {kind}"
        ));
    }

    if command == "source" {
        let sub = rest.first().map(String::as_str).unwrap_or("");
        if sub == "move" || sub == "rename" {
            op.insert("op".to_string(), Data::Str("source.move".to_string()));
            op.insert(
                "from".to_string(),
                Data::Str(rest.get(1).cloned().unwrap_or_default()),
            );
            op.insert(
                "to".to_string(),
                Data::Str(rest.get(2).cloned().unwrap_or_default()),
            );
            return Ok(Data::Map(op));
        }
        if sub == "delete" || sub == "remove" {
            op.insert("op".to_string(), Data::Str("source.delete".to_string()));
            op.insert(
                "path".to_string(),
                Data::Str(rest.get(1).cloned().unwrap_or_default()),
            );
            return Ok(Data::Map(op));
        }
        return Err(format!("未知の source サブコマンドです: {}", rest.join(" ")));
    }

    if command == "fix" || command == "fmt" {
        op.insert("op".to_string(), Data::Str("fix".to_string()));
        if let Some(t) = rest.first().cloned().or_else(|| flag_string(flags, "table")) {
            op.insert("table".to_string(), Data::Str(t));
        }
        return Ok(Data::Map(op));
    }

    if command == "record" {
        let sub = rest.first().map(String::as_str).unwrap_or("");
        let table_name = rest
            .get(1)
            .cloned()
            .or_else(|| flag_string(flags, "table"))
            .unwrap_or_default();
        let table = find_table(snapshot, &table_name)?;

        if sub == "add" {
            let assignment_tokens = if rest.len() > 2 { &rest[2..] } else { &[] };
            let assignments = parse_assignments(snapshot, &table.fields, assignment_tokens)?;
            op.insert("op".to_string(), Data::Str("record.add".to_string()));
            op.insert("table".to_string(), Data::Str(table_name));
            if let Some(s) = flag_string(flags, "source").or_else(|| flag_string(flags, "file")) {
                op.insert("source".to_string(), Data::Str(s));
            }
            if let Some(payload_raw) =
                flag_string(flags, "data").or_else(|| flag_string(flags, "record"))
            {
                let parsed = parse_structured_data(&payload_raw)?;
                if matches!(parsed, Data::Array(_)) {
                    op.insert("records".to_string(), parsed);
                } else {
                    op.insert("values".to_string(), parsed);
                }
            }
            if !assignments.is_empty() {
                let assign_data: Vec<Data> = assignments
                    .into_iter()
                    .map(|(steps, val)| {
                        let mut m = IndexMap::new();
                        m.insert("pathStr".to_string(), Data::Str(path_steps_to_str(&steps)));
                        m.insert("value".to_string(), val);
                        Data::Map(m)
                    })
                    .collect();
                op.insert("assignments".to_string(), Data::Array(assign_data));
            }
            if let Some(tags) = flag_string(flags, "tags") {
                op.insert(
                    "tags".to_string(),
                    Data::Array(split_csv(&tags).into_iter().map(Data::Str).collect()),
                );
            }
            if let Some(ak) = flag_bool(flags, "auto-key") {
                op.insert("autoKey".to_string(), Data::Bool(ak));
            }
            return Ok(Data::Map(op));
        }

        if sub == "set" {
            let mut pk = flag_string(flags, "pk");
            let mut assign_start = 2;
            let where_flags = flag_strings(flags, "where");
            let all_flag = flag_bool(flags, "all").unwrap_or(false);
            if pk.is_none() && where_flags.is_empty() && !all_flag {
                if let Some(arg2) = rest.get(2) {
                    if !arg2.contains('=') {
                        pk = Some(arg2.clone());
                        assign_start = 3;
                    }
                }
            }
            let assignment_tokens = if rest.len() > assign_start {
                &rest[assign_start..]
            } else {
                &[]
            };
            let assignments = parse_assignments(snapshot, &table.fields, assignment_tokens)?;
            op.insert("op".to_string(), Data::Str("record.set".to_string()));
            op.insert("table".to_string(), Data::Str(table_name));
            if let Some(pk_val) = pk {
                op.insert("pk".to_string(), Data::Str(pk_val));
            }
            if !where_flags.is_empty() {
                op.insert(
                    "where".to_string(),
                    Data::Array(where_flags.into_iter().map(Data::Str).collect()),
                );
            }
            if all_flag {
                op.insert("all".to_string(), Data::Bool(true));
            }
            if let Some(s) = flag_string(flags, "source").or_else(|| flag_string(flags, "file")) {
                op.insert("source".to_string(), Data::Str(s));
            }
            if let Some(patch_raw) =
                flag_string(flags, "patch").or_else(|| flag_string(flags, "data"))
            {
                let set_obj = parse_structured_data(&patch_raw)?;
                op.insert("set".to_string(), set_obj);
            }
            if !assignments.is_empty() {
                let assign_data: Vec<Data> = assignments
                    .into_iter()
                    .map(|(steps, val)| {
                        let mut m = IndexMap::new();
                        m.insert("pathStr".to_string(), Data::Str(path_steps_to_str(&steps)));
                        m.insert("value".to_string(), val);
                        Data::Map(m)
                    })
                    .collect();
                op.insert("assignments".to_string(), Data::Array(assign_data));
            }
            if let Some(tags) = flag_string(flags, "tags") {
                op.insert(
                    "tags".to_string(),
                    Data::Array(split_csv(&tags).into_iter().map(Data::Str).collect()),
                );
            }
            let add_tags: Vec<String> = flag_strings(flags, "add-tag")
                .into_iter()
                .flat_map(|s| split_csv(&s))
                .collect();
            if !add_tags.is_empty() {
                op.insert(
                    "addTags".to_string(),
                    Data::Array(add_tags.into_iter().map(Data::Str).collect()),
                );
            }
            let rem_tags: Vec<String> = flag_strings(flags, "remove-tag")
                .into_iter()
                .flat_map(|s| split_csv(&s))
                .collect();
            if !rem_tags.is_empty() {
                op.insert(
                    "removeTags".to_string(),
                    Data::Array(rem_tags.into_iter().map(Data::Str).collect()),
                );
            }
            return Ok(Data::Map(op));
        }

        if sub == "delete" || sub == "remove" {
            let mut pk = flag_string(flags, "pk");
            let where_flags = flag_strings(flags, "where");
            let all_flag = flag_bool(flags, "all").unwrap_or(false);
            if pk.is_none() && where_flags.is_empty() && !all_flag {
                if let Some(arg2) = rest.get(2) {
                    pk = Some(arg2.clone());
                }
            }
            op.insert("op".to_string(), Data::Str("record.delete".to_string()));
            op.insert("table".to_string(), Data::Str(table_name));
            if let Some(pk_val) = pk {
                op.insert("pk".to_string(), Data::Str(pk_val));
            }
            if !where_flags.is_empty() {
                op.insert(
                    "where".to_string(),
                    Data::Array(where_flags.into_iter().map(Data::Str).collect()),
                );
            }
            if all_flag {
                op.insert("all".to_string(), Data::Bool(true));
            }
            if let Some(s) = flag_string(flags, "source").or_else(|| flag_string(flags, "file")) {
                op.insert("source".to_string(), Data::Str(s));
            }
            return Ok(Data::Map(op));
        }
    }

    Err(format!("未知のコマンドです: {command}"))
}

#[derive(Serialize, Deserialize)]
struct LatestBuildFile {
    #[serde(rename = "workspaceDigest")]
    workspace_digest: String,
    #[serde(rename = "builtAt")]
    built_at: String,
    result: BuildResult,
}

pub fn run_cli(argv: &[String], options: CliOptions) -> CliResult {
    let cwd = options
        .cwd
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let parsed = parse_args(argv);
    let want_json = flag_bool(&parsed.flags, "json").unwrap_or(false);
    let dry_run = flag_bool(&parsed.flags, "dry-run").unwrap_or(false);
    let strict = flag_bool(&parsed.flags, "strict").unwrap_or(false);
    let command = parsed.positionals.first().map(String::as_str).unwrap_or("");
    let rest: Vec<String> = if parsed.positionals.len() > 1 {
        parsed.positionals[1..].to_vec()
    } else {
        Vec::new()
    };

    let ok = |payload: IndexMap<String, Data>, text_summary: &str| -> CliResult {
        let mut body = IndexMap::new();
        body.insert("ok".to_string(), Data::Bool(true));
        for (k, v) in payload {
            body.insert(k, v);
        }
        let data = Data::Map(body);
        CliResult {
            exit_code: 0,
            stdout: if want_json {
                format!("{}\n", stringify_json(&data))
            } else {
                format!("{text_summary}\n")
            },
            stderr: String::new(),
        }
    };

    let fail = |err: CommitError| -> CliResult {
        let mut body = IndexMap::new();
        body.insert("ok".to_string(), Data::Bool(false));
        body.insert("error".to_string(), Data::Str(err.message.clone()));
        if let Some(probs) = err.problems {
            body.insert("problems".to_string(), Data::Array(probs));
        }
        let data = Data::Map(body);
        CliResult {
            exit_code: 1,
            stdout: if want_json {
                format!("{}\n", stringify_json(&data))
            } else {
                String::new()
            },
            stderr: if want_json {
                String::new()
            } else {
                format!("{}\n", err.message)
            },
        }
    };

    let run_inner = || -> Result<CliResult, CommitError> {
        if command.is_empty()
            || command == "help"
            || flag_bool(&parsed.flags, "help").unwrap_or(false)
        {
            let mut payload = IndexMap::new();
            payload.insert("help".to_string(), Data::Str(HELP_TEXT.to_string()));
            return Ok(ok(payload, HELP_TEXT.trim_end()));
        }

        if command == "init" {
            let target_dir = cwd.join(rest.first().map(String::as_str).unwrap_or("."));
            let target_dir = fs::canonicalize(&target_dir).unwrap_or(target_dir);
            let project_name = flag_string(&parsed.flags, "name")
                .unwrap_or_else(|| {
                    target_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                })
                .trim()
                .to_string();
            if project_name.is_empty() {
                return Err(CommitError::from(
                    "プロジェクト名を --name で指定してください。".to_string(),
                ));
            }
            let toml_path = target_dir.join(PROJECT_FILE_NAME);
            if fs::symlink_metadata(&toml_path).is_ok() {
                return Err(CommitError::from(format!(
                    "すでに {PROJECT_FILE_NAME} が存在します: {}",
                    toml_path.display()
                )));
            }
            fs::create_dir_all(target_dir.join("sources"))
                .map_err(|e| CommitError::from(e.to_string()))?;
            let cfg_text =
                write_config(&default_config(&project_name), None).map_err(CommitError::from)?;
            fs::write(&toml_path, cfg_text).map_err(|e| CommitError::from(e.to_string()))?;

            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("init".to_string()));
            payload.insert(
                "root".to_string(),
                Data::Str(target_dir.to_string_lossy().to_string()),
            );
            payload.insert(
                "projectFile".to_string(),
                Data::Str(toml_path.to_string_lossy().to_string()),
            );
            payload.insert("name".to_string(), Data::Str(project_name.clone()));
            return Ok(ok(
                payload,
                &format!(
                    "プロジェクト「{project_name}」を作成しました: {}",
                    toml_path.display()
                ),
            ));
        }

        let project_file = resolve_project_file(
            flag_string(&parsed.flags, "project").as_deref(),
            &cwd,
        )
        .map_err(CommitError::from)?;
        let (mut workspace, project) = load_workspace(&project_file).map_err(CommitError::from)?;
        let mut session = EditSession::new(&project);

        if command == "inspect" {
            let target = rest
                .first()
                .cloned()
                .or_else(|| flag_string(&parsed.flags, "target"));
            let info = inspect_project(&session.snapshot, target.as_deref())
                .map_err(CommitError::from)?;
            if let Some(t) = target {
                let summary = stringify_json(&info);
                let mut payload = IndexMap::new();
                payload.insert("command".to_string(), Data::Str("inspect".to_string()));
                payload.insert(
                    "root".to_string(),
                    Data::Str(workspace.root.to_string_lossy().to_string()),
                );
                payload.insert("target".to_string(), Data::Str(t));
                payload.insert("detail".to_string(), info);
                return Ok(ok(payload, &summary));
            }
            let tables_list = info
                .get("tables")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let types_list = info
                .get("types")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let t_desc: Vec<String> = tables_list
                .iter()
                .map(|t| {
                    format!(
                        "{}[{}]",
                        t.get("name").map(|v| v.text()).unwrap_or_default(),
                        t.get("recordCount").map(|v| v.text()).unwrap_or_default()
                    )
                })
                .collect();
            let ty_desc: Vec<String> = types_list
                .iter()
                .map(|t| {
                    format!(
                        "{}({})",
                        t.get("name").map(|v| v.text()).unwrap_or_default(),
                        t.get("kind").map(|v| v.text()).unwrap_or_default()
                    )
                })
                .collect();
            let total_probs = info
                .get("problems")
                .and_then(|p| p.get("total"))
                .map(|v| v.text())
                .unwrap_or_else(|| "0".to_string());
            let syn_probs = info
                .get("problems")
                .and_then(|p| p.get("syntaxErrorCount"))
                .map(|v| v.text())
                .unwrap_or_else(|| "0".to_string());
            let summary_lines = [
                format!(
                    "Project: {} ({})",
                    project.snapshot.config.name,
                    workspace.root.display()
                ),
                format!(
                    "Tables ({}): {}",
                    tables_list.len(),
                    if t_desc.is_empty() {
                        "なし".to_string()
                    } else {
                        t_desc.join(", ")
                    }
                ),
                format!(
                    "Types ({}): {}",
                    types_list.len(),
                    if ty_desc.is_empty() {
                        "なし".to_string()
                    } else {
                        ty_desc.join(", ")
                    }
                ),
                format!("Problems: {total_probs} (syntaxErrors: {syn_probs})"),
            ];
            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("inspect".to_string()));
            payload.insert(
                "root".to_string(),
                Data::Str(workspace.root.to_string_lossy().to_string()),
            );
            if let Data::Map(imap) = info {
                for (k, v) in imap {
                    payload.insert(k, v);
                }
            }
            return Ok(ok(payload, &summary_lines.join("\n")));
        }

        if command == "validate" {
            let profile = flag_string(&parsed.flags, "profile").unwrap_or_default();
            let problems = format_problems(&session.snapshot, &profile).map_err(CommitError::from)?;
            let profile_data = if profile.is_empty() {
                Data::Null
            } else {
                Data::Str(profile.clone())
            };
            if !problems.is_empty() {
                let lines: Vec<String> = problems
                    .iter()
                    .map(|p| {
                        let code = p.get("code").map(|v| v.text()).unwrap_or_default();
                        let file = p.get("file").map(|v| v.text()).unwrap_or_default();
                        let path_str = p
                            .get("path")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .map(|x| x.text())
                                    .collect::<Vec<_>>()
                                    .join(".")
                            })
                            .unwrap_or_default();
                        let msg = p.get("message").map(|v| v.text()).unwrap_or_default();
                        format!("[{code}] {file} ({path_str}): {msg}")
                    })
                    .collect();
                let mut body = IndexMap::new();
                body.insert("ok".to_string(), Data::Bool(false));
                body.insert("command".to_string(), Data::Str("validate".to_string()));
                body.insert("profile".to_string(), profile_data);
                body.insert(
                    "problemCount".to_string(),
                    Data::Int(problems.len() as i128),
                );
                body.insert("problems".to_string(), Data::Array(problems.clone()));
                let data = Data::Map(body);
                return Ok(CliResult {
                    exit_code: 1,
                    stdout: if want_json {
                        format!("{}\n", stringify_json(&data))
                    } else {
                        String::new()
                    },
                    stderr: if want_json {
                        String::new()
                    } else {
                        format!(
                            "検証エラーが {} 件見つかりました:\n{}\n",
                            problems.len(),
                            lines.join("\n")
                        )
                    },
                });
            }
            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("validate".to_string()));
            payload.insert("profile".to_string(), profile_data);
            payload.insert("problemCount".to_string(), Data::Int(0));
            payload.insert("problems".to_string(), Data::Array(Vec::new()));
            let suffix = if profile.is_empty() {
                String::new()
            } else {
                format!(", profile: {profile}")
            };
            return Ok(ok(payload, &format!("検証OK（0件のエラー{suffix}）")));
        }

        if command == "config" && rest.first().map(String::as_str).unwrap_or("show") == "show" {
            let cfg_data = crate::wire::config_to_data(&session.snapshot.config);
            let summary = stringify_json(&cfg_data);
            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("config.show".to_string()));
            payload.insert("config".to_string(), cfg_data);
            return Ok(ok(payload, &summary));
        }

        if command == "record"
            && matches!(rest.first().map(String::as_str), Some("list") | Some("get"))
        {
            let sub = rest[0].as_str();
            let table_name = rest
                .get(1)
                .cloned()
                .or_else(|| flag_string(&parsed.flags, "table"))
                .unwrap_or_default();
            let pk = if sub == "get" {
                rest.get(2)
                    .cloned()
                    .or_else(|| flag_string(&parsed.flags, "pk"))
            } else {
                flag_string(&parsed.flags, "pk")
            };
            if sub == "get" && pk.is_none() {
                return Err(CommitError::from(
                    "取得するレコードの主キーを指定してください。".to_string(),
                ));
            }
            let qopts = QueryOptions {
                pk: pk.clone().map(Data::Str),
                r#where: flag_strings(&parsed.flags, "where"),
                source: flag_string(&parsed.flags, "source")
                    .or_else(|| flag_string(&parsed.flags, "file")),
                profile: flag_string(&parsed.flags, "profile"),
                limit: flag_string(&parsed.flags, "limit").and_then(|s| s.parse::<usize>().ok()),
                offset: flag_string(&parsed.flags, "offset").and_then(|s| s.parse::<usize>().ok()),
            };
            let result = query_records(&session.snapshot, &table_name, &qopts)
                .map_err(CommitError::from)?;
            let records = result
                .get("records")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if sub == "get" && records.is_empty() {
                return Err(CommitError::from(format!(
                    "主キーに一致するレコードが見つかりません: {}",
                    pk.unwrap_or_default()
                )));
            }
            if sub == "get" {
                let first = &records[0];
                let rec = first.get("record").cloned().unwrap_or(Data::Null);
                let src = first.get("source").cloned().unwrap_or(Data::Null);
                let summary = stringify_json(&rec);
                let mut payload = IndexMap::new();
                payload.insert("command".to_string(), Data::Str("record.get".to_string()));
                payload.insert("table".to_string(), Data::Str(table_name));
                payload.insert("source".to_string(), src);
                payload.insert("record".to_string(), rec);
                return Ok(ok(payload, &summary));
            } else {
                let summary = stringify_json(&Data::Array(records));
                let mut payload = IndexMap::new();
                payload.insert("command".to_string(), Data::Str("record.list".to_string()));
                if let Data::Map(rmap) = result {
                    for (k, v) in rmap {
                        payload.insert(k, v);
                    }
                }
                return Ok(ok(payload, &summary));
            }
        }

        if command == "tx" || command == "apply" {
            let raw_input = if let Some(file_opt) = flag_string(&parsed.flags, "file") {
                fs::read_to_string(cwd.join(file_opt))
                    .map_err(|e| CommitError::from(e.to_string()))?
            } else if let Some(ops_opt) = flag_string(&parsed.flags, "ops") {
                ops_opt
            } else if let Some(stdin_str) = options.stdin {
                stdin_str
            } else {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .map_err(|e| CommitError::from(e.to_string()))?;
                buf
            };
            if raw_input.trim().is_empty() {
                return Err(CommitError::from(
                    "tx に渡す操作リスト（JSONまたはYAML）が空です。".to_string(),
                ));
            }
            let parsed_payload =
                parse_structured_data(&raw_input).map_err(CommitError::from)?;
            let ops: Vec<Data> = if let Data::Array(arr) = parsed_payload {
                arr
            } else if let Some(Data::Array(arr)) = parsed_payload.get("operations") {
                arr.clone()
            } else {
                return Err(CommitError::from(
                    "tx の入力は操作オブジェクトの配列 `[...]` または `{ operations: [...] }` で指定してください。".to_string(),
                ));
            };
            let mut results = Vec::with_capacity(ops.len());
            for op in &ops {
                results.push(apply_operation(&mut session, op).map_err(CommitError::from)?);
            }
            let (committed, changed_len, prob_len) =
                commit_session(&mut workspace, &session, dry_run, strict)?;
            let op_count = results.len();
            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("tx".to_string()));
            payload.insert("operationCount".to_string(), Data::Int(op_count as i128));
            payload.insert("results".to_string(), Data::Array(results));
            for (k, v) in committed {
                payload.insert(k, v);
            }
            return Ok(ok(
                payload,
                &format!(
                    "トランザクション完了（{op_count}操作, 変更ファイル: {changed_len}件, 現在の診断エラー: {prob_len}件）"
                ),
            ));
        }

        if command == "build" {
            let profile = flag_string(&parsed.flags, "profile").unwrap_or_default();
            let result =
                build_workspace(&workspace, &session.snapshot, &profile, 1)
                    .map_err(CommitError::from)?;
            let latest_file = safe_path(&workspace.root, ".masterdata/latest-build.json")
                .map_err(CommitError::from)?;
            if let Some(parent) = latest_file.parent() {
                fs::create_dir_all(parent).map_err(|e| CommitError::from(e.to_string()))?;
            }
            let latest_payload = LatestBuildFile {
                workspace_digest: compute_workspace_digest(&project),
                built_at: String::new(),
                result: result.clone(),
            };
            fs::write(
                &latest_file,
                serde_json::to_string_pretty(&latest_payload).unwrap(),
            )
            .map_err(|e| CommitError::from(e.to_string()))?;

            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("build".to_string()));
            payload.insert(
                "result".to_string(),
                crate::wire::build_result_to_data(&result),
            );
            return Ok(ok(
                payload,
                &format!(
                    "Build成功（tables: {}, records: {}, output: {}）",
                    result.tables, result.records, result.directory
                ),
            ));
        }

        if command == "publish" {
            let should_build = flag_bool(&parsed.flags, "build").unwrap_or(false);
            let profile = flag_string(&parsed.flags, "profile").unwrap_or_default();
            let latest_file = safe_path(&workspace.root, ".masterdata/latest-build.json")
                .map_err(CommitError::from)?;
            let current_digest = compute_workspace_digest(&project);

            let build_result = if should_build {
                let res = build_workspace(&workspace, &session.snapshot, &profile, 1)
                    .map_err(CommitError::from)?;
                if let Some(parent) = latest_file.parent() {
                    fs::create_dir_all(parent).map_err(|e| CommitError::from(e.to_string()))?;
                }
                let latest_payload = LatestBuildFile {
                    workspace_digest: current_digest,
                    built_at: String::new(),
                    result: res.clone(),
                };
                fs::write(
                    &latest_file,
                    serde_json::to_string_pretty(&latest_payload).unwrap(),
                )
                .map_err(|e| CommitError::from(e.to_string()))?;
                res
            } else {
                let latest_text = optional_read(&latest_file)
                    .map_err(CommitError::from)?
                    .ok_or_else(|| {
                        CommitError::from("先に `masterdata build` を実行してください。".to_string())
                    })?;
                let latest: LatestBuildFile = serde_json::from_str(&latest_text)
                    .map_err(|e| CommitError::from(e.to_string()))?;
                if latest.workspace_digest != current_digest {
                    return Err(CommitError::from(
                        "最後のBuild以降にソースまたは設定が変更されています。先に `masterdata build` を再実行するか `masterdata publish --build` を使用してください。".to_string(),
                    ));
                }
                latest.result
            };

            let targets = PublishConfig {
                csharp: flag_string(&parsed.flags, "csharp")
                    .unwrap_or_else(|| session.snapshot.config.publish.csharp.clone()),
                binary: flag_string(&parsed.flags, "binary")
                    .unwrap_or_else(|| session.snapshot.config.publish.binary.clone()),
            };
            let message = publish_build(&build_result, &workspace.root, &targets)
                .map_err(CommitError::from)?;
            let mut tmap = IndexMap::new();
            tmap.insert("csharp".to_string(), Data::Str(targets.csharp));
            tmap.insert("binary".to_string(), Data::Str(targets.binary));
            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str("publish".to_string()));
            payload.insert("targets".to_string(), Data::Map(tmap));
            payload.insert(
                "buildDirectory".to_string(),
                Data::Str(build_result.directory),
            );
            payload.insert("message".to_string(), Data::Str(message.clone()));
            return Ok(ok(payload, &message));
        }

        if matches!(
            command,
            "config" | "table" | "type" | "rename" | "source" | "fix" | "fmt" | "record"
        ) {
            let op = build_cli_operation(&session, command, &rest, &parsed.flags)
                .map_err(CommitError::from)?;
            let op_name = op.get("op").map(|v| v.text()).unwrap_or_default();
            let result = apply_operation(&mut session, &op).map_err(CommitError::from)?;
            let (committed, changed_len, prob_len) =
                commit_session(&mut workspace, &session, dry_run, strict)?;
            let mut payload = IndexMap::new();
            payload.insert("command".to_string(), Data::Str(op_name.clone()));
            payload.insert("result".to_string(), result);
            for (k, v) in committed {
                payload.insert(k, v);
            }
            return Ok(ok(
                payload,
                &format!(
                    "{op_name} 完了（変更ファイル: {changed_len}件, 現在の診断エラー: {prob_len}件）"
                ),
            ));
        }

        Err(CommitError::from(format!("未知のコマンドです: {command}")))
    };

    match run_inner() {
        Ok(res) => res,
        Err(err) => fail(err),
    }
}

pub fn run_stdio_host() {
    use crate::service::ProjectService;
    use std::io::{BufRead, Write};
    let service = ProjectService::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            eprintln!("Invalid native request: {line}");
            continue;
        };
        let id = req["id"].as_u64().unwrap_or(0);
        let method = req["method"].as_str().unwrap_or("");
        let payload = req["payload"].as_str().unwrap_or("[[],[]]");
        let response = match service.request_wire(method, payload) {
            Ok(encoded) => json!({ "id": id, "payload": encoded }),
            Err(error) => json!({ "id": id, "error": error }),
        };
        let _ = writeln!(stdout, "{response}");
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("masterdata-rust-cli-{name}-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn cli_init_inspect_tx_and_ulong_precision() {
        let root = temp_dir("init-tx");
        let root_str = root.to_string_lossy().to_string();

        let init = run_cli(
            &[
                "init".to_string(),
                root_str.clone(),
                "--name".to_string(),
                "RustCLI".to_string(),
                "--json".to_string(),
            ],
            CliOptions {
                cwd: Some(root.clone()),
                stdin: None,
            },
        );
        assert_eq!(init.exit_code, 0, "{}", init.stderr);

        let tx_ops = r#"[
          {"op":"table.create","name":"account","fields":[{"key":0,"name":"id","type":"ulong"},{"key":1,"name":"code","type":"string"}],"pk":["id"]},
          {"op":"record.add","table":"account","records":[{"id":"18446744073709551614","code":"007"}]}
        ]"#;
        let tx = run_cli(
            &["tx".to_string(), "--json".to_string()],
            CliOptions {
                cwd: Some(root.clone()),
                stdin: Some(tx_ops.to_string()),
            },
        );
        assert_eq!(tx.exit_code, 0, "{}{}", tx.stderr, tx.stdout);

        let add = run_cli(
            &[
                "record".to_string(),
                "add".to_string(),
                "account".to_string(),
                "code=008".to_string(),
                "--json".to_string(),
            ],
            CliOptions {
                cwd: Some(root.clone()),
                stdin: None,
            },
        );
        assert_eq!(add.exit_code, 0, "{}{}", add.stderr, add.stdout);
        assert!(add.stdout.contains("18446744073709551615"));

        let val = run_cli(
            &["validate".to_string(), "--json".to_string()],
            CliOptions {
                cwd: Some(root.clone()),
                stdin: None,
            },
        );
        assert_eq!(val.exit_code, 0, "{}{}", val.stderr, val.stdout);

        let _ = fs::remove_dir_all(&root);
    }
}

