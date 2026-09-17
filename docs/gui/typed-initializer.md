# GUI仕様: Typed Migration Initializer

Status: Approved

この仕様はTable Editor / Type EditorのAdd operationで使用するschema-aware initializer editorを定義する。Migrationのinitializer要否、validity、Plan / Apply semanticsは既存のSchema Migration / Type Migration ownerが保持する。適用記録は[仕様変更0016](../spec-changes/0016-desktop-daily-editing.md)を参照する。

## 規範要件

### GUI-TABLE-INT-008

AddField initializerはshared resolved value authoring modelからschema-aware controlを構成しなければならない（MUST）。通常入力にraw YAML / JSONを要求してはならない（MUST NOT）。型/modifier変更時は古いinitializerを別型へcoerceせずunsetへ戻し、既存Planを失効させる。
未入力とexplicit nullを区別し、required initializerが未入力/invalidならMigration Planのpreconditionとして扱う（MUST）。Data Editorのinvalid Save許可をMigration initializer validityへ転用してはならない（MUST NOT）。64-bit、nested値、valid constantの意味は既存ownerに従う。

### GUI-TYPE-INT-010

AddCustomField initializerへGUI-TABLE-INT-008と同じshared editor / unsetとnullの区別 / Plan失効を適用しなければならない（MUST）。initializer要否とvalidityはType Migration ownerが決め、data draftのplaceholderを自動的なinitializerへ昇格させてはならない（MUST NOT）。

## 既存Editor contractへの適用

`GUI-TABLE-INT-001`および`GUI-TYPE-INT-003`が要求するinitializer inputを本仕様のtyped editorで具体化する。Migration semantics、destructive authorization、Plan / Diff / stale-plan safetyは変更しない。

## 受け入れ証拠

nested 64-bit値、unset/null区別、type/modifier変更でPlan失効、invalid initializerとData Editorのvalidation-nonblocking Saveを混同しないことを検証する。
