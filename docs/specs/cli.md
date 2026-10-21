# CLI Authoring and Automation

Status: Approved

Approval mode: Agent-autonomous
Basis: 2026-10-20〜2026-10-21 HumanのCLI設計決定とRust採用指示。別passでAIエージェントファーストの操作体系、順序・YAML AST・64bit整数・`$tags` 保持、一部破損・診断エラー時の通過と品質ゲートの境界、複数ファイル追加先と古いBuildのPublish防止、RustネイティブCLIとGUIの責務境界をchallenge reviewし、Blockingなし。Human gate: None within delegated scope。

Change provenance: [Rust CLIとネイティブバックエンドコアの採用](../spec-changes/0013-rust-cli-and-backend-core.md)。

## 根拠と範囲

2026-10-20〜2026-10-21 Human requestに基づく、GUIとは別のCLIプロダクトの仕様。

- **Human Decision / Requirement**:
  - YAMLデータ構造、プロジェクト概念（`masterdata.toml`、`sources/`、`.masterdata/`）、診断・編集コア、Build / PublishをGUIと共有するCLIをRustで提供する。
  - 主な使い手はAIエージェント（Claude Code等）およびCI / 自動化スクリプトとし、AIエージェントファーストで設計する。
  - `init`、`build`、`publish` に加え、レコード編集とスキーマ・型定義（テーブル、列、キー、参照、Enum / Flags / Custom / Value Object）の編集・リネーム・構造把握・検証を安全かつアトミックに行えるようにする。
  - GUIの表示順に直結する配列順（`fields`、`custom.fields`、`enum.members`、`flags.members`、`records`）やYAML AST（コメント・クオート・既存キー順・改行コード・64bit整数精度・`$tags`・未変更ファイル）を勝手にソート・破壊しない。
  - プロジェクトに一部破損や診断エラーがある場合の扱い：実行に支障がない調査・編集コマンドは通し、品質ゲート（`validate` / `build` / `publish`）および変更対象自体が構文破損している場合のみエラーとして止める。
- **Agent Decision**:
  - GUIの0ms即時編集レスポンスを守るためWebView内のインメモリdraftセッションは維持しつつ、CLI実行バイナリ（`masterdata`）とDesktopのネイティブバックエンド（永続化・検証・Build / Publish）を `src-tauri` のRustコアで共有実装する。
  - SQLの更新文（`UPDATE` / `INSERT` / `ALTER`）は多段ネスト構造・`Flags`・`$tags`・分離データファイル・YAMLコメント保持と衝突するため採用せず、AIエージェントの直接YAML編集を補助するコマンド（`inspect` / `validate` / `rename` / `fix`）と、構造化編集コマンド（`config` / `table` / `type` / `record`）および一括トランザクション（`tx`）で構成する。
  - 条件絞り込みはシェルのエスケープ負荷が低い `--pk` とドットパス対応の `--where`（およびJSON指定）を採用する。
  - 複数ソースを持つテーブルへのレコード追加は、[Desktop Authoring UI-004](desktop.md#ui-004--対象の境界と直接操作) と同じく追加先ファイル（`--source`）の明示を必須とし、暗黙の推測追加を行わない。
  - `build` と `publish` を別プロセスで実行できるよう `.masterdata/latest-build.json` にワークスペースのダイジェストを記録し、ソース変更後の古い成果物の誤Publishを防止する。

## Non-scope

GUI固有の画面状態・タブ・アニメーション、SQLクエリエンジンの同梱、Gitのcommit / branch操作、旧YAMLプロジェクト設定の自動移行、Unityエディタ起動。

## CLI-001 — プロジェクト解決と共通入出力

- コマンド入口はRust製の単一ネイティブバイナリ `masterdata <subcommand>` とし、実行時にNode.jsランタイムを要求しない。
- `--project <path>` で `masterdata.toml` ファイルまたはその親フォルダを指定できる。省略時はカレントディレクトリから親方向へ `masterdata.toml` を探索する。`masterdata.toml` 以外の任意のTOML / YAMLファイルが指定された場合は拒否する。
- `masterdata init [dir] [--name <name>]` は対象フォルダに `masterdata.toml` と `sources/` を作成する。既存の `masterdata.toml` がある場合は上書きせず失敗する。
- 全サブコマンドは `--json` を受け付け、成功・失敗・変更ファイル・診断件数を機械可読なJSONで出力する。`long` / `ulong` の64bit整数（`BigInt`）は精度を失わずにシリアライズ・パースする。

## CLI-002 — エラー耐性と品質ゲートの分離

1. **プロジェクト設定（`masterdata.toml`）の破損**:
   - ファイル欠落、TOML構文不正、`name` 欠落、既知項目（`profiles`, `publish`）のshape不正は、`init` 以外の全コマンドで即座に非0終了とし、ファイルを一切変更しない。
2. **YAMLソース（`sources/*.yaml`）の構文エラー（`syntaxError`）**:
   - `inspect` はエラーで停止せず、正常にパースできたテーブル・型定義を返しつつ、構文エラーのファイルを診断情報として報告する（終了コード `0`）。
   - 編集コマンド（`table`, `type`, `record`, `rename`, `fix`, `tx`）は、**書き換え対象となるファイル自体**（連鎖更新が波及する定義・データファイルを含む）が構文エラーの場合のみ、データ破壊を防ぐため変更を中止して非0終了とする。**無関係な別ファイル**が構文エラーであっても、そのファイルには触れず対象の変更を完了させる。
3. **意味上の診断エラー（型不一致、キー重複、`MasterRef` 参照切れ、`Flags` 不正、型循環等）**:
   - `inspect` および編集コマンド（`config`, `table`, `type`, `record`, `rename`, `fix`, `tx`）は、意味エラーが存在していてもブロックせず処理を通し（終了コード `0`）、出力に現在の未解決診断件数を含める。
   - 編集コマンドおよび `tx` に `--strict` が指定された場合は、操作適用後の状態に1件でも構文・意味エラーがあればディスクへの保存を行わず非0終了でロールバックする。
   - `validate`、`build`、`publish` は品質ゲートとして、対象範囲に1件でも構文・意味エラーがあれば非0終了で失敗させる。

## CLI-003 — 構造把握と検証（inspect / validate）

- `masterdata inspect [target] [--project <path>] [--json]`:
  - `target` 省略時は `records` 本体を展開せず、プロジェクト設定、全テーブルの概要（定義パス、列定義、主キー、副キー、参照、レコード総数、データファイル一覧、次に自動採番可能な主キー候補）、全型の概要（種類、定義パス、基底型、候補・フィールド、`Flags` の次の単一bit値）、および現在の診断エラー件数と構文エラーファイルを出力する。
  - `target` にテーブル名または型名を指定した場合は、その定義の詳細、参照元・参照先テーブル、既存主キー一覧（または候補一覧）を出力する。未定義の名前は非0終了とする。
- `masterdata validate [--profile <name>] [--project <path>] [--json]`:
  - `--profile` 指定時はそのプロファイルで選択されるレコードを対象に、未指定時は全レコードを対象に診断を実行する。
  - 各問題のファイルパス（`sources/...`）、位置パス、エラーコード、メッセージを出力し、0件なら終了コード `0`、1件以上または未定義プロファイルなら終了コード `1` とする。

## CLI-004 — 順序・AST・既存値を壊さない編集とリネーム

- すべての変更系コマンドは、保存前に変更対象ファイルの外部変更とパス衝突を検査し、`.masterdata/transactions/` にバックアップを残すトランザクションでアトミックに書き込む。`--dry-run` 指定時はディスクを書き換えず、変更されるファイル一覧と診断結果のみを返す。
- 順序とYAML ASTの保持：
  - `fields`、`custom.fields`、`enum.members`、`flags.members`、`records` の配列順を明示的な並替え操作なしにソートしない。既存フィールドの `key`（MessagePack番号）を振り直さない。
  - 変更された有効なYAMLソースは、空のMap / Sequenceを `{}` / `[]`、非空を2スペースのblock形式（`lineWidth: 0`）で出力し、既存のコメント・引用符・Mapキー順・改行コード（CRLF / LF）・64bit整数精度・`$tags`・型変更後の既存不整合値を保持する。未変更のファイルは書き換えない。
- 横断リネーム（`masterdata rename`）：
  - `rename table <OldName> <NewName>`: テーブル定義、全分離データファイルの `table` 指定、他テーブルからの `MasterRef` の `target.table` を一括更新する。
  - `rename type <OldName> <NewName>`: 型定義名、全テーブル・Custom構造体フィールドの `type` 指定を一括更新する。
  - `rename field <TableOrCustom> <OldField> <NewField>`: 定義内のフィールド名、全レコード（またはネスト構造体）の該当キー、主キー・副キー・自テーブルおよび他テーブルからの `MasterRef` のフィールド指定を一括更新する。
  - `rename member <EnumOrFlags> <OldMember> <NewMember>`: 候補名と、その型を使用する全レコード（配列・ネスト構造体内を含む）の値を一括更新する。`Flags` の予約候補 `None` はリネーム・削除・0以外の値への変更を拒否する。
- 設定・スキーマ・型定義の操作：
  - `config`: `show`、`set`（`--name`, `--publish-csharp`, `--publish-binary`）、`profile set <name>`（`--include`, `--exclude`）、`profile remove <name>`。
  - `table`:
    - `create <Table> [--path <relpath>] [--pk <fields>]`: テーブル定義を作成する。
    - `delete <Table>`: テーブル定義と所属する分離データファイルを削除する。
    - `data-file create <Table> <relpath>`: 空の分離データファイル（`kind: data`）を作成する。
    - `column add <Table> <name> <type> [--array] [--nullable] [--key <n>]`: 次の空き `key` を自動採番（明示指定可）して列を追加し、既存レコードに型の初期値を補完する。
    - `column set <Table> <name> [--type <type>] [--array <bool>] [--nullable <bool>] [--key <n>]`: 列定義を変更する。既存レコードの値は変換・破棄しない。
    - `column remove <Table> <name>` / `column rename <Table> <old> <new>` / `column move <Table> <name> --to <index>`。
    - `key set-pk <Table> <fields>` / `key add-sk <Table> <fields> [--non-unique]` / `key remove-sk <Table> <index>`。
    - `ref add <Table> <name> --fields <ownFields> --target-table <Target> --target-fields <targetFields> [--csharp-name <name>]` / `ref remove <Table> <name>`。
  - `type`:
    - `create <kind> <Name> [--path <relpath>] [--underlying <type>]`: `enum` / `flags` / `custom` / `valueObject` を作成する（`flags` は `None = 0` を初期保持）。
    - `delete <Name>` / `set-underlying <Name> <underlying>`。
    - `member add <Name> <member> [--value <n>]`（省略時は `enum` は最大値+1、`flags` は未使用の最小単一bitを自動採番） / `member set <Name> <member> --value <n>` / `member remove <Name> <member>` / `member rename <Name> <old> <new>` / `member move <Name> <member> --to <index>`。
    - `field add <Name> <field> <type> [--array] [--nullable] [--key <n>]` / `field set` / `field remove` / `field rename` / `field move`。
  - `fix [--table <Table>] [--dry-run] [--strict] [--json]`:
    - 配列順や既存値を変えずに、未設定の `key` 番号補完、レコードで欠落（`undefined`）しているフィールドへの初期値補完、および変更対象の2スペースblock YAML整形を行う。

## CLI-005 — レコード編集と一括トランザクション（record / tx）

- レコードの特定と絞り込み：
  - `--pk <value>`: 単一主キーは値そのまま（例：`--pk 101`）、複合主キーは `k1=v1,k2=v2` またはJSONオブジェクトで指定する。
  - `--where <expr>`: ドット＋配列インデックスパスおよび `$tags` に対応する比較条件（`=`, `!=`, `>`, `>=`, `<`, `<=`, `~=`, `in`）。複数指定時はAND条件とする。
- `masterdata record list <Table>` / `masterdata record get <Table> <pk>`:
  - `--where`、`--pk`、`--source <path>`、`--profile <name>` でレコードを抽出する。
- `masterdata record add <Table> [field=value ...] [--json <payload>] [--source <path>] [--tags <t1,t2>]`:
  - 未指定のフィールドは `defaultValue` で再帰的に初期値を生成する。
  - 主キー（または一意な副キー）が未指定の場合は、全データソースを横断して重複しない次のキー値を自動採番する。明示指定されたキーが既存レコードと重複する場合はエラーとする。
  - 対象テーブルのソースが1ファイルのみならそのファイルへ追加する。複数ソースが存在するテーブルで `--source` が省略された場合は、候補ファイル一覧を示して拒否する。
- `masterdata record set <Table> [--pk <pk> | --where <expr>] [path=value ...] [--json <patch>] [--add-tag <t>] [--remove-tag <t>] [--tags <t1,t2>]`:
  - ドット＋インデックスパス（例：`rewards[0].count=3`、`stats.hp=120`）またはJSONパッチで対象レコードを更新する。対象が0件の場合は非0終了とする。
- `masterdata record delete <Table> [--pk <pk> | --where <expr> | --all]`:
  - 条件に一致するレコードを削除する。指定がない場合や対象が0件の場合は非0終了とする。
- `masterdata tx [--file <path>] [--dry-run] [--strict] [--json]`:
  - 標準入力または `--file` からJSON / YAMLの操作配列を読み込み、1つのメモリdraftセッション上で順番に実行し、全操作が成功したときだけ1回のトランザクションで保存する。途中の操作が1つでも失敗した場合はディスクへ一切書き込まず非0終了とする。

## CLI-006 — BuildとPublish

- `masterdata build [--profile <name>] [--project <path>] [--json]`:
  - 指定プロファイル（省略時は全レコード）で検証し、C#コード生成・.NETコンパイル・MasterMemoryバイナリ生成・全選択レコードのreload検証を実行する。
  - 成功時は `.masterdata/builds/<id>/build.json` に加え、`.masterdata/latest-build.json` にビルド結果とワークスペース状態（`masterdata.toml` と全 `sources/` のダイジェスト）を記録する。
- `masterdata publish [--build] [--profile <name>] [--csharp <dir>] [--binary <file>] [--project <path>] [--json]`:
  - `--build` 指定時は先に `build` を実行する。未指定時は `.masterdata/latest-build.json` を参照し、ビルド未実行またはビルド後にソース・設定が変更されていればエラーとして拒否する。
  - 出力先は引数指定があればそれを優先し、省略時は `masterdata.toml` の `publish` 設定を使う。管理下の生成ファイルのみを更新・退役し、未管理ファイル・`.meta`・`masterdata.toml`・`sources/` を保護する。

## Acceptance / Validation

- `init` でプロジェクト作成 → `type` / `table` / `record` および `tx` でEnum・Flags・Custom・ValueObject・複合キー・`MasterRef`・分離データファイルを含むデータを構築 → `inspect` と `validate` で確認。
- 既存のコメント・クオート・`fields` / `members` / `records` の配列順・`$tags`・`ulong`（$2^{53}$ 超の64bit整数）精度・未変更ファイルが編集後も維持されることを確認。
- 複数ソースを持つテーブルで `--source` なしの `record add` が拒否され、`--source` 指定で該当ファイルにのみ追加されることを確認。
- 一部ファイルにYAML構文エラーや意味エラーがある状態で、`inspect` や無関係ファイルの編集は通り、構文エラーファイル自体への変更や `validate` / `build`（および `--strict` 指定時の編集）は非0終了で止まりデータが保護されることを確認。
- `rename` でテーブル名・列名・型名・Enum/Flags候補名を変更し、分離データ・ネスト構造体・`MasterRef` まで一括更新されることを確認。
- `build` で実.NETによるMasterMemoryバイナリ生成とreloadを確認し、ソース変更後の古いBuildに対する `publish` 拒否と、最新Buildの `publish`（未管理ファイルと `.meta` の保護）を確認。
