# 仕様変更: Project conventions、Settings scope、public generate removal

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。0012はHuman Approval後に
     canonical ownerへ適用済みである。 -->

## Affected Specifications

- `docs/specs/cli.md` — `Status: Approved`、public command surface、`CLI-002`、historical `CLI-004`、
  `CLI-011`のcanonical owner。0012のdeltaを適用済み
- `docs/specs/project-layout.md` — `Status: Approved`、recommended project convention、
  settings scope、`init` scaffold、tool-state ownershipのcanonical owner。0012のRequirementを適用済み
- `docs/specs/README.md` — non-canonical index。apply時のstatus/index consistencyに影響し得る
- `docs/spec-changes/0011-cli-surface-and-schema-migration.md` — `Status: Applied`のhistorical
  record。今回変更しない

0012は、Human maintainerによる明示的Approval後に、承認されたdeltaだけをatomicにcanonical ownerへ
適用したspecification changeである。`docs/spec-changes/0011-cli-surface-and-schema-migration.md`は
historical Applied recordとして変更していない。

## 適用した差分の背景（Applied Delta Context）

MasterDataはYAMLをcanonical source of truthとし、full buildがcoherent canonical artifact setを生成し、
publishが既存artifact setを配布する。0011で定義された`generate`のmaterialization先は未決定のままで
あり、`generate`専用のpublic commandを残すと、canonical artifact setの外側へC#だけをmaterializeする
二重経路を新たに維持することになる。

このchangeで承認・適用されたscopeは次のとおりである。

- **Approved decision**: public CLI surfaceから`masterdata generate`を削除し、C# generation capabilityは
  full build内部stageとして維持する。
- **Approved decision**: recommended human-facing source layoutをkind-first conventionとして記録する。
- **Approved decision**: Project Settings、Project Tool State、User Settings / UI Stateを分離する。
- **Constraint**: source directory名、directory tree、file pathはTable/type/index identityを決めない。
- **Constraint**: User Settings / UI Stateはcanonical source interpretation、validation、Build Selection、
  generated C#、binary、artifact identity、publish configurationを変更してはならない。
- **Constraint**: `.masterdata/`はdefaultではtool-owned derived stateであり、YAML source authorityやUser
  Settings storageへ昇格させない。
- **Deferred decision**: preview/export UX、concrete init warning、placeholder strategy、custom pathのGit policy、
  user settings storage schemaなどの実装・product detailは今回固定しない。

## 適用した差分（Applied Delta）

### 1. Public CLI surfaceからのgenerate removal

canonical public CLI surfaceを、次の6つへ変更した。

```text
masterdata init
masterdata doctor
masterdata validate
masterdata build [--publish]
masterdata publish
masterdata migrate
```

`masterdata generate`はpublic CLI commandとして削除する。`masterdata export`を代替commandとして追加しては
ならない（MUST NOT）。将来GUI/WebでGenerated C# Previewまたはexplicit C# Exportを提供する余地は残すが、
そのUX、destination、filename、CLI surfaceは今回決めない。

public commandを削除しても、C# generation capabilityを削除してはならない（MUST NOT）。full buildは引き続き、
少なくとも次の内部pipelineを使用する。

```text
YAML source
→ validation / semantic resolution
→ C# code generation
→ .NET / MasterMemory builder
→ coherent canonical artifact set
```

通常のproject-local C# materializationはfull buildが生成するcanonical artifact set内の
`.masterdata/output/csharp/`を使用する。`generate → .masterdata/generated/`、`build →
.masterdata/output/`という二重materialization modelは採用しない。

このdeltaは、C# codegen内部stage、`build`、`publish`、`build --publish`のApproved semanticを変更しない。
standalone `publish`はcurrent YAMLをparse、validate、freshness-check、implicit buildせず、既存のreceipt-valid
canonical artifact setを入力とする。

### 2. Existing CLI Requirement IDのtraceability

このchangeは既存Requirement IDをrename、reassign、または再利用しない。canonical applyでは次のdeltaを
適用した。

| Requirement | Applied treatment | Boundary |
| --- | --- | --- |
| `CLI-002` | canonical command setから`generate`を除き、7 commandから6 commandへ精密化する。`build [--publish]`はbuild commandの正式surfaceとして表現する。 | `publish`、`build --publish`、`migrate`の既存責務は維持する。 |
| `CLI-004` | generate専用requirementとしてhistorical identityを保持する。current canonical behaviorは定義しない。 | 別semanticへreassignまたは再利用しない。 |
| `CLI-011` | source-derived staged operationの対象を`validate`と`build`へ変更する。`generate`を前段対象から除く。 | buildのsource resolve、parse、semantic validation / resolutionをskipしない原則は維持する。publish semanticsは変更しない。 |

`CLI-004`はformalな新lifecycle vocabularyを追加せず、historical tombstoneとして`cli.md`に残した。
0011が定義した旧generate contractと0012の適用をtraceできるが、current normative behaviorではない。

### 3. Recommended project layout

次のlayoutをdefault/recommended human-facing conventionとしてcanonicalizeした。

```text
my-game-master-data/
├─ masterdata.toml
├─ .gitignore
├─ sources/
│  ├─ schemas/
│  ├─ types/
│  └─ data/
│     └─ ...
└─ .masterdata/
   ├─ output/
   │  ├─ csharp/
   │  ├─ masterdata.bytes
   │  └─ .masterdata-artifact-set.json
   └─ cache/
```

`masterdata.toml`とconfigured source rootsはproject/configuration contractである。一方、
`sources/schemas/`、`sources/types/`、`sources/data/`はhuman-facing organization conventionであり、
directory name自体にsemantic identityを与えてはならない（MUST NOT）。既存`PROJECT-006`を維持し、YAML
documentの`kind`、`table`、type declarationなどのdocument contentをauthorityとする。

`sources/data/`以下は大量dataを整理するため任意にnestしてよい。例えば
`sources/data/item/base.yaml`と`sources/data/item/event-summer.yaml`は許容されるorganization例だが、`item/` directoryからTable
identityを推測してはならない。domain/table-first layoutを禁止するsemantic ruleは追加しない。

`.masterdata/generated/`および`.masterdata/generated/csharp/`はrecommendedまたはcanonical layoutへ
導入しない。`.masterdata/output/`はfull buildのcoherent artifact set、`.masterdata/cache/`はtool stateとして
扱うが、`[build] artifact_dir`と`cache`の既存configurabilityは維持する。

### 4. Settings scope

Projectに関係する設定を次の3 scopeへ分離した。

#### Project Settings

`masterdata.toml`をcurrent v1のsingle canonical configuration entrypointとする。Project Settingsはshared、
reproducible、project-scopedで、通常Git trackedである。

例:

- project identity
- source roots
- build artifact/cache paths
- Build Profiles
- publish targets
- project semanticsへ影響するfuture compiler/schema options

同じcanonical sourceと同じexplicit operationのsemantic、build、publish resultへ影響するvalueはUser Settingsから
取得してはならず（MUST NOT）、Project Settingsとして扱わなければならない（MUST）。`.masterdata/settings.toml`
などのsecond semantic settings layerや、user-local overrideでbuild resultを変える設計は今回導入しない。
将来のconfig composition/include mechanismは別specification changeで扱う。

#### Project Tool State

`.masterdata/**`をdefault namespaceとするproject-local tool-owned stateである。derived/reconstructable where
applicableであり、canonical source authorityではない。少なくとも`.masterdata/output/`と`.masterdata/cache/`を
含む。

`.masterdata/`をUser Settings storageとして扱ってはならない。`build.artifact_dir`と`build.cache`がconfigurableで
ある既存Approved contractは維持し、このchangeによってdefault namespaceをhard-codeしたり、custom pathを自動的に
Git ignoreしたりしない。

#### User Settings / UI State

User SettingsとUI Stateはproject source treeではなくhost/user-local storageへ置く。Native/DesktopではOSまたは
application user-data storage、Webではhost-local browser storageを使用できるが、exact technologyは固定しない。

例:

- theme、language、recent projects
- last selected table、panel sizes、column widths
- sort/filter UI、expanded tree

これらを`masterdata.toml`または`.masterdata/**`のcanonical/default project storageへ置いてはならない
（MUST NOT）。project-specific UI stateのkeyを`project.id`またはopaque local workspace/bookmark identityへ
紐付けるmechanismは今回固定しない。既存Web/Native Hostのopaque identityとraw path非公開原則を維持する。

User Settings / UI Stateは、次を変更してはならない（MUST NOT）。

- canonical source interpretation
- validation result semantics
- Build Selection semantics
- generated C# semantics
- binary semantics
- canonical artifact-set identity
- publish target configurationまたはpublish semantics

source自体が持つpresentation semantics（例えばschema field declaration order）はUser Settingsへ移さず、既存の
source/domain ownerのままとする。

### 5. `.masterdata/` Git policy

default recommended Git policyは次のentryを`.gitignore`へ含めることである。

```gitignore
/.masterdata/
```

通常trackedとするのは`masterdata.toml`とcanonical YAML sources、defaultではignoreするのは`.masterdata/**`
である。canonical artifact setにおける「canonical」はbuild/publish artifact authorityを意味し、YAML source
authorityを意味しない。`.masterdata/output/`をGit source authorityへ昇格させてはならない（MUST NOT）。

custom `build.artifact_dir`または`build.cache` pathに対するautomatic Git ignore policyは今回一般化しない。

### 6. `init`のobservable behavior

`masterdata init`はdefault scaffoldとして、次を作成する。

- `masterdata.toml`
- `sources/`
- `sources/schemas/`
- `sources/types/`
- `sources/data/`

`.gitignore`が存在しない場合に限り`.gitignore`を作成し、少なくとも`/.masterdata/`を含める。既存`.gitignore`
はinitが自動編集、append、rewriteしてはならない（MUST NOT）。既存fileがある場合のwarning/recommendation UXは
Open Questionとして残す。

`init`は`.masterdata/`、`.masterdata/output/`、`.masterdata/cache/`をeager-createしてはならない（MUST NOT）。
必要なOperationは、初めて必要とした時に、既存Approved path safetyに従ってlazy-createする（MUST）。
validation-only operationはartifact output directoryを作る必要がない。

`sources/schemas/`、`sources/types/`、`sources/data/`がempty directoryとしてGitで追跡されないことへの対応として、
`.gitkeep`、placeholder YAML、READMEを自動生成するsemanticは決めていない。placeholderがsource semanticsを誤って
示さないよう、empty-directory placeholder strategyはdeferred decisionとして残る。

## 既存Approved build / publish semanticsとの境界

このchangeは次を変更しない。

```text
build
→ coherent canonical artifact set
→ .masterdata/output/（default）

publish
→ existing receipt-valid canonical artifact set
→ external targets

build --publish
→ build success only
→ publish
```

standalone publishはcurrent YAML freshnessを検証しない。C#だけを単独更新して`.masterdata/output/`をpartial
stateにするOperationも追加しない。C# generationはfull build内部stageとして維持する。

## 互換性（Compatibility）

- 現行の`masterdata init`、CLI parser、project discovery、build/publish runtimeはこのdocs-only applyでは変更しない。
- `project-info`はcurrent implementationから削除しない。canonical CLI surfaceから`generate`を除くdeltaとは独立している。
- `masterdata generate`のpublic removalは新しい`masterdata export`へのrenameではない。
- Existing `masterdata.toml`のproject/source/build/publish configuration、`artifact_dir`、`cache`のconfigurabilityを維持する。
- source directory conventionを採用しても、既存`PROJECT-006`、Table/Data/Type、YAML subset、Build Selection semanticsを変更しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

このApplied changeに対応する以下のevidenceはfuture planned evidenceである。実装済み・pass済みとは扱わない。

- CLI helpに`generate`がcanonical commandとして存在しないことを確認する。
- full build内部のC# generationと`.masterdata/output/csharp/` materializationを維持する。
- `validate` / `build`のsource-derived前段skip禁止とstandalone publishのsource-freshness非依存を確認する。
- `init`がkind-first source scaffoldを作ることを確認する。
- source directory名やnested pathがTable/type/index identityを変更しないことを確認する。
- `init`が`.masterdata/`、`output/`、`cache/`をeager-createしないことを確認する。
- `.gitignore`不存在時だけ`/.masterdata/`を含むfileを作成することを確認する。
- existing `.gitignore`をinitが無断rewrite/appendしないことを確認する。
- User Settings変更がbuild semantics、artifact identity、publish configurationを変更しないことを確認する。
- `.masterdata/output/`と`.masterdata/cache/`がcanonical YAML source authorityにならないことを確認する。
- `CLI-004`のhistorical identityを保持し、別semanticへ再利用しないことを確認する。
- `PROJECT-CONVENTION-001`がrecommended source organizationとsemantic identityを分離することを確認する。
- `PROJECT-CONFIG-007`がProject / Tool / User SettingsのscopeとUser Settingsのsemantic isolationを守ることを確認する。
- `PROJECT-CONFIG-008`がsource scaffold、missing-only `.gitignore`、existing `.gitignore` preservation、
  `.masterdata` lazy creationを守ることを確認する。

### Implementation Gap

Human Approvalとcanonical apply後も、次は実装gapとして残る。

- canonical six-command surfaceへのCLI parser/help更新（public `generate` removalを含む）
- `init`のrecommended scaffold、`.gitignore`生成、lazy tool-state creation
- source convention directoriesのempty-directory UX
- user-local User Settings / UI State storage boundary
- future Generated C# Preview / explicit C# Export UX

このchange自体はruntime implementationではない。実装はApproved canonical ownerのRequirementを使用し、
このchangeに記録されたfuture evidenceを満たす必要がある。

## Applied Requirement IDs

次のRequirement IDをcanonical ownerへ追加・適用した。

| Requirement ID | Canonical owner / meaning |
| --- | --- |
| `PROJECT-CONVENTION-001` | `sources/schemas/`、`sources/types/`、`sources/data/`をkind-first human-facing conventionとして扱い、directory/pathからsemantic identityを導出しない。 |
| `PROJECT-CONFIG-007` | Project Settings（`masterdata.toml`）、Project Tool State（default `.masterdata/**`）、User Settings / UI Stateのscopeを分離し、User Settingsがproject/build/publish semanticsを変更しない。 |
| `PROJECT-CONFIG-008` | `init`のdefault scaffold、missing-only `.gitignore` generation、既存`.gitignore`を自動rewrite/appendしない境界、`.masterdata`のlazy creationを定義する。empty source convention directoryの保持方法は定義しない。 |

`CLI-002`、`CLI-011`は既存Requirement IDの精密化として適用し、`CLI-004`はhistorical traceabilityを
保持する。`CLI-004`を別のRequirementへreassignまたは再利用しない。既存Requirementのnumberingは詰め直さない。

## 未解決事項（Deferred Decisions）

- Generated C# Preview / explicit C# ExportのUX、destination、filename、将来CLI surface
- `masterdata init`のexisting `.gitignore`に対するwarning/recommendation UX
- empty source convention directoryの`.gitkeep`、placeholder YAML、README strategy
- source filename / data filename naming convention
- custom artifact/cache pathに対するautomatic Git ignore policy
- User Settings serialization schema、Desktop user-data physical path、browser storage technology
- project-specific UI stateのkey algorithm
- config include/fragments、named source groups、ignore patterns
- source symlink product policy
- `CLI-004`のtombstone presentation、deprecation/versioning、およびCLI helpでのhistorical visibility
- Formatter、Migration、Build Selection、Table、Type、YAMLの既存semanticの追加変更

これらはcanonical apply後も未決定であり、implementation convenienceのために暗黙に決定してはならない。
Recommended directory conventionはpresentation/organization ruleであり、semantic identity ruleではない。

## レビュー（Review）

- Human Approval: 実施済み。Reviewed revision `a7296c8da14bf229ebc26306056257fbe793df26`について、
  final reviewでBlockingなしを確認した後、Human maintainerが明示的に「次に進む」と回答した。
- canonical apply: 実施済み。`docs/specs/cli.md`と`docs/specs/project-layout.md`へ承認済みdeltaを適用した。
- `docs/specs/README.md`はstatus consistencyの変更が不要なため変更していない。
- `docs/spec-changes/0011-cli-surface-and-schema-migration.md`はhistorical Applied recordとして変更していない。
- production Rust、CLI parser、tests、fixtures、build/publish runtimeは変更していない。
- apply前にCI run `34001004579`がreviewed revisionと同じSHAに対してcompleted / successであることを確認した。
- 0012の適用はこのexplicit approvalを根拠とし、commitまたはpushからApprovalを推測したものではない。

## 承認記録（Approval Record）

Human maintainerは、Reviewed revision `a7296c8da14bf229ebc26306056257fbe793df26`の0012 proposalを
最終reviewし、Blockingなしを確認した後、「次に進む」と明示的に回答した。これを0012のHuman Approval
として記録する。Approvalはcommitまたはpushから推測したものではない。

承認・適用scopeは次のとおりである。

- public CLIから`generate`を削除すること
- `CLI-002`を6-command canonical surfaceへ精密化すること
- `CLI-004`をhistorical tombstoneとして保持し、別semanticへ再利用しないこと
- `CLI-011`を`validate` / `build`のsource-derived operationへ精密化すること
- C# codegen capabilityをfull build内部stageとして維持すること
- `.masterdata/generated/`を採用しないこと
- kind-first source conventionを推奨すること
- source path/directoryをsemantic identityにしないこと
- Project Settings / Project Tool State / User Settingsの3-scope separation
- `.masterdata/**`をdefaultのproject-local tool-owned derived stateとすること
- `/.masterdata/`をdefault Git ignore policyとすること
- `init`のsource scaffold
- existing `.gitignore`を自動編集しないこと
- `.masterdata` tool stateをlazy-createすること
- User Settingsがbuild/project/publish semanticsへ影響しないこと

empty-directory placeholder strategy（`.gitkeep`、README、placeholder YAML、その他のplaceholder）は
承認scopeに含まれず、未決定のまま残る。
