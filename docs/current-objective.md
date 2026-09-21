# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Reference v1を定義・実装し、Table間relationshipをcanonical YAMLで宣言し、selected logical dataset上でshared Rust semanticsによるintegrity validationとC# helper生成を行い、Desktop authoringから扱えるverification済みcandidateへ到達する。**

## Completion slices

### Reference semantics

- [Index / Reference](specs/index-and-reference.md)へReference v1 Option Bのcore semanticsを適用し、source declaration、target identity、cardinality、nullability、missing target、Build Selectionとの順序を確定する。
- Primary Key / Secondary Keyの既存identityを再利用し、MessagePack `key`やgenerated `indexNo`をReference identityへ昇格させない。
- source/targetの型・component order・selected dataset上のintegrityをshared Rust coreで検証する。

### Generated API / Desktop authoring

- Reference helperはmaster recordへ`MemoryDatabase`を保持せずcallerから受け取り、MasterMemoryのgenerated Table query APIへlowerする。
- Table EditorからReference declarationをraw YAML手編集へ戻らず扱えるようにし、frontendへReference resolution semanticsを複製しない。source-preserving add/edit/removeは既存Plan/Apply boundaryを使う。

### Verification

- scalar/composite、Primary/Secondary、unique/non-unique、Build Selection、missing target、generated C# compileをfocused evidenceで確認する。
- repository checks、fresh review、exact Candidateのrequired remote CI reconciliationを完了する。

## Explicit non-scope

- released-version compatibility policy、Reference-aware migration / automatic rewrite。
- cross-project Reference、runtime mutable relationship、binaryからのReference推論。
- P5 expression / computed / programmable view、Git product integration。
- Approved specificationにないobservable behaviorをimplementation convenienceで追加すること。

## Audit

このObjectiveは2026-09-21 JSTにHumanが次priorityとしてReference方向へ進むことを選択し、仕様変更0023でOption B（Nullable Referenceをv1へ含める）を明示採用した。generated helper public method namingとOptional non-unique return contractは未解決のHuman gateとしてDevelopment Stateが所有する。
