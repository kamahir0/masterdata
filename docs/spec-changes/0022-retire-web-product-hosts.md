# 仕様変更: Browser / Web product hostのretire

Status: Applied

## Affected Specifications

- [Product Vision](../product/vision.md): active hostをCLI / Tauri Desktopに限定する。
- [Runtime hosts](../specs/runtime-hosts.md): `RUNTIME-HOST-001..016`をcurrent implementation authorityからretireする。
- [CLI surface](../specs/cli.md) `CLI-010`、[GUI app shell](../gui/app-shell.md) `GUI-SHELL-CAPABILITY-001`、[Project layout](../specs/project-layout.md) `PROJECT-CONFIG-007`、[GUI Source Creation](../gui/source-creation/spec.md) `GUI-CREATE-STATE-003`、source authoring / migration specifications: Web前提のroutingを既存Desktop/CLI ownerへ戻す。
- [RFC 0004](../rfcs/0004-web-native-host-runtime.md)、[ADR 0006](../adr/0006-host-capability-composition.md): historical decisionとしてcurrent authorityから外す。
- [Current Objective](../current-objective.md)、[Development State](../execution-state.md): Browser Save decision待ちを終了し、このretirementを完了境界にする。

## 根拠と分類（Source Evidence and Classification）

- Human Decision: 2026-09-21 JST、Standalone Web、Connected Web、Browser Host、Native Host、Web向けWASM runtime、静的Web配布を現在および予見可能なproduct scopeから外す。read-onlyまたはexperimentalなWebは残さない。
- Constraint: CLI / Desktopのobservable behaviorとYAML source authority、shared Rust semantics、.NET / MasterMemory delegation、migration / Build / Publish safetyを維持する。
- Agent Decision: 5e0d5b6でcoreへ移したData / Table / Type snapshotとData queryは、native applicationも現在利用するpure projectionなので保持する。Browser-specific adapter、WASM build/ABI、Browser-only testsは削除する。
- Historical evidence: 0009 / 0010のApplied audit recordとRFC 0004の当時のalternative比較はGit historyと文書に残す。Draft 0021は`Rejected`にする。

## 提案する差分（Proposed Delta）

- Runtime hosts仕様を`Deprecated`にし、`RUNTIME-HOST-001..016`を再利用しないhistorical IDとして保持する。Web / Native Host requirementをactive normative contractとして残さない。
- CLI direct in-process application呼び出しは`CLI-010`と[ADR 0002](../adr/0002-rust-core-shared-by-cli-and-gui.md)が所有する。domain/application、Tauri adapter、.NET delegationはADR 0002 / 0003と各canonical specへrouteし、Web host abstractionを再導入しない。
- DesktopのSave / Build / Recovery Requiredの既存動作を保ち、Web由来のhost capability wordingとBrowser workspace permission wordingを除く。shared source patch derivationとnative commit I/Oの分離は保持する。
- Product VisionとREADMEのcurrent architectureからWeb / Native Hostを削除する。Web対応を未実装項目、roadmap、disabled featureとして残さない。
- Cargo / npm / xtask / CIのWeb専用surfaceを削除する。Desktop WebView / WebDriverに必要な依存とtestは保持する。

## 互換性（Compatibility）

CLI / Desktopのpublic command、GUI操作、canonical YAML、config、binary/artifact formatを変更しない。Browser prototypeのstatic bundle / WASM APIは未releaseのretired surfaceであり、提供を終了する。Requirement IDは歴史上の識別子として保持し、別意味に再割当てしない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- Browser/WASM/static buildのproduction code、test、CI、dependencyがcurrent treeから除去される。
- CLI / Desktop、Data/Table/Type authoring、migration、Build / Publish、.NET bridgeのfocusedおよびrepository checksが通る。
- active docs / codeへのWeb host要求の残留を検索し、historical recordだけを明示的に区別する。
- exact Candidateをfresh reviewし、required remote CIをreconcileする。

## 未解決事項（Open Questions）

None. Browser Saveの残余raceはproduct scope撤回によりdecision不要。

## レビュー（Review）

Blocking Issues: None. Non-blocking Issues: None. Questions: None. Humanがproduct scope変更を明示決定済みであり、CLI / Desktopの契約を保持する。Requirement IDはretire後も再利用しない。canonical owner、互換性、回帰対象が特定され、提案どおり適用可能。

## 承認記録（Approval Record）

Approval mode: Human. Basis: 2026-09-21 JSTの本task依頼におけるBrowser / Web / Native Hostの明示的retire決定。Agentが新たなproduct scopeを選択したものではない。

Application: 上記canonical ownersへ適用。exact commitはGit historyで追跡する。
