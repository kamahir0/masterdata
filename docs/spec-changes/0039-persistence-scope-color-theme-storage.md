# 仕様変更: Persistence scopeとColor Theme storage authority

Status: Applied

## Affected Specifications

- `docs/specs/project-layout.md` — `PROJECT-CONFIG-007`
- `docs/gui/color-theme/spec.md` — `GUI-THEME-002`, `GUI-THEME-004`

## 根拠と分類（Source Evidence and Classification）

- Human Decision: 設定を「Projectごとに`masterdata.toml`へ書くshared setting」「Projectごとに`.masterdata/**`へ書くlocal setting/state」「Project非依存でApplication Support相当へ書くapplication setting」の3区分として整理する。
- Human Request: Color Theme仕様をこの区分へ合わせて修正する。
- Existing Approved Authority: `PROJECT-CONFIG-007`はProject Settings / Project-local tool state / User Settingsの分離をすでに所有しているため、別specを増やさず同Requirementをcanonical ownerとして更新する。
- Agent Decision: 3区分を`Project Settings` / `Project Local State` / `Application User Settings / UI State`と命名する。
- Agent Decision: DesktopのProject非依存Application User SettingsはOS標準per-user application data/config領域をcanonical authorityとし、WebView storageをdurable authorityから除外する。exact file名/serializationは未固定とする。
- Agent Decision: current `localStorage`実装との不一致が生じるためColor Theme canonical statusを`Implemented`から`Approved`へ戻す。実装変更は今回のspec-only requestのscope外とする。
- Agent Decision: legacy WebView valueはone-time migration sourceとして許可し、実装時に既存preferenceを保持できるようにする。

## 確定事項（Confirmed Decisions）

- Project Settings: `masterdata.toml`をcanonical entrypointとするshared/reproducible semantic config。
- Project Local State: `.masterdata/**`をdefault namespaceとするproject-scoped / local-only state。semantic/build/publish resultを変更しない。
- Application User Settings / UI State: Project非依存のuser-local preference/state。DesktopではOS標準per-user application data/config領域へ保存する。
- ThemeはApplication User Settingsに属するため`.masterdata/**`へ保存しない。
- 将来Project固有Theme overrideを導入する場合はProject Local State側で別途仕様化する。

## 提案する差分（Proposed Delta）

### `PROJECT-CONFIG-007`

- 上記3 scopeを明示し、resultへ影響するvalueはProject Settingsだけに置く。
- Project Local Stateへproject-specific UI/tool stateを置けるが、shared semanticsを変更してはならない。
- Project非依存Application User Settingsのcanonical storageをOS側application data/config領域に固定する。
- WebView/browser storageをdurable Application Settings authorityとして扱うことを禁止する。
- exact file名 / serialization / subdirectory layoutは必要なfeature implementationへ委ねる。

### Color Theme

- `GUI-THEME-002`をApplication User Settingsへrouteし、Theme preferenceをOS側Application storageへ保存する。
- `GUI-THEME-004`のstartup restore sourceをcanonical Application preference storageとする。
- React/WebView層がdurable persistence authorityを所有しないarchitecture boundaryを明示する。
- future Project-specific overrideはProject Local Stateとして別途仕様化する。
- current implementationが新contractを満たすまでcanonical statusを`Approved`とする。

## 互換性（Compatibility）

ProjectのYAML/TOML、CLI、build/publish、generated artifact、public schema formatは変更しない。
user-local persistence backendは後続implementationで変更対象になるが、legacy WebView valueをmigration sourceとして
許可するため既存Theme preferenceを保持できる。migrationのexact mechanismはimplementation ownerへ委ねる。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- 同一source/operationのsemantic、build、publish resultはProject Local/Application stateで変わらない。
- Project固有local stateは`.masterdata/**`へ保存できるが、Project共有semantic configとして扱わない。
- ThemeはProject切替をまたいで維持され、Project/YAML/Project Settings dirty stateを発生させない。
- canonical Theme preferenceはProject tree / `.masterdata/**` / WebView storageではなくApplication storageから復元される。
- current frontend `localStorage` persistenceはcanonical contractを満たさないため、後続implementationでnative/Application preference storageへ移す必要がある。

## 未解決事項（Open Questions）

- Project Local State / Application User Settingsのexact file名、serialization format、subdirectory layout。

## レビュー（Review）

### Blocking Issues
None identified.

### Non-blocking Issues
- Current Theme implementationは新しいcanonical persistence authorityと不一致であり、後続implementation Objectiveが必要。

### Questions
None identified.

### Approved as Proposed
Yes.

### Autonomous approval eligibility
- Eligible: Yes
- Human gate: None
- Rationale: Humanが3つの保存scopeとColor Theme仕様修正を明示している。Project/CLI/public config formatは変更せず、legacy preferenceのmigration pathも閉じない。今回はspec-onlyでimplementation mutationを行わない。

### Review dimensions
- Intent fidelity: Humanが示した3区分をそのままProject Settings / Project Local State / Application User Settingsへ対応。
- Internal consistency: semantic resultへ影響する設定をProject Settingsへ限定。
- Cross-spec consistency: persistence scopeは`PROJECT-CONFIG-007`、Theme固有behaviorはColor Themeがowner。
- Normative strength: scope/authorityはMUST、exact file layoutは未固定。
- Testability: storage location category、Project dirty非干渉、startup restoreを検証可能。
- Backward compatibility: Project/public contractは不変。legacy Theme valueはmigration sourceとして保持可能。
- Unresolved ambiguity: physical formatだけexplicit Open Question。
- Implementation leakage: platform-standard application storage boundaryだけを固定。
- Documentation ownership: 新しい重複specを作らず既存ownerを更新。

## 承認記録（Approval Record）

- Approval mode: Agent-autonomous
- Basis: Human-selected persistence classification and explicit spec correction request
- Review result: Blockingなし、material ambiguityなし、Human gateなし
- Canonical application: `PROJECT-CONFIG-007`, `GUI-THEME-002`, `GUI-THEME-004`, Color Theme architecture/statusへ反映
