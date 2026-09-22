# Authoring Query仕様

Status: Approved

Domain: Authoring

この仕様は、Desktop制作v1のData Editor / Table Overviewで使用するread-only search、filter、sortと保存済みOverview datasetを定義する。適用記録は[仕様変更0016](../spec-changes/0016-desktop-daily-editing.md)および[0017](../spec-changes/0017-desktop-workspace-settings.md)を参照する。
[Computed View](computed-view.md)のresolved scalarも、同じtyped query capabilityへlowerされる。Computed Viewのparser/type/evaluation semanticsは同仕様が所有し、この仕様はquery operator capabilityだけを所有する。

## 規範要件

### AUTHORING-QUERY-001

filter / search / sortはshared layerがtyped snapshotから導出し、source bytes、dirty、canonical record orderingを変えてはならない（MUST NOT）。初期状態はqueryなし、source順でなければならない（MUST）。File Viewはcurrent buffer、Overviewは保存済みsnapshotを使う。

検索は全top-level valid non-null scalarの表示textに対するcase-sensitive部分一致のORとする。literal string/Enum symbol、bool小文字、数値はAUTHORING-BATCH-004のcopy textを使用する。query emptyは検索制約なし。complex、invalid、nullのtextを検索一致と推測しない（MUST NOT）。

### AUTHORING-QUERY-002

column filterは複数条件のANDとし、以下のoperatorだけをv1で提供しなければならない（MUST）。operator/input不正はquery failureで、旧結果をcurrentとして表示してはならない（MUST NOT）。query stateはproject source/configへ保存しない。

| Shape | Operator |
| --- | --- |
| int / uint / long / ulongとnumeric Value Object | equals, not-equals, less-than, greater-than |
| string / string Value Object | equals, not-equals, contains（ordinal、case-sensitive、normalizationなし） |
| bool | equals, not-equals |
| normal Enum | symbol equals, not-equals |
| 全field | is-null, is-invalid |

通常operatorはvalid non-null値だけを評価し、それ以外はnot-equalsもfalse。is-nullは明示null/Added placeholder、is-invalidはshared field validationがinvalidと判定した値に一致する。Required nullは両方に一致しなければならない（MUST）。query入力自体は対象typeのvalid non-null値を要求する。type unresolved等でfield validityを確定できない場合、is-invalidを推測せずそのqueryをUnavailableとする。検索制約とcolumn filter群はANDで合成する。
float/doubleの数値filterとFlags / Array / Customの構造queryはv1外とし、数値comparison capabilityを暗黙追加しない。

### AUTHORING-QUERY-003

表示sortは単一columnのascending / descending / noneとし、比較可能なint / uint / long / ulong / string / Value Objectだけに提供しなければならない（MUST）。comparisonはApproved Primitive / Value Object ownerに従う。valid値、null、その他invalidの順を両方向で保ち、descendingはvalid値内だけを反転する。同値と各非valid群は入力source順でstableとする（MUST）。
Enum / bool / float / double / complexのsortはv1外。schema column順をview独自順に変えない。

### AUTHORING-OVERVIEW-001

shared applicationは保存済みconfig、source membership、対象Tableと解決に必要なschema/type/data bytesを識別するsnapshotを取得しなければならない（MUST）。read-only queryはこのsnapshotとresult identityを返し、frontendはfilesystem discovery / YAML parseを行わない。
読込中の変更を検出した場合は結果をcurrent completeとして返さず、再読込要求として扱う（MUST）。filesystem全体のglobal atomic snapshotを保証する意味ではない。
各rowはTable identityとsource provenanceを持ち、同じPKでもdeduplicateしない。default orderはproject-relative source pathのdeterministic順、そのfile内source順。PKによる並べ替えはAUTHORING-QUERY-003の明示queryだけで行う。

### AUTHORING-OVERVIEW-002

Overviewはdomain-invalid recordを黙って捨ててはならない（MUST NOT）。安全にproject/targetをresolveできるrangeを表示し、parse不能・membership不明・type unresolved等で欠落があればincompleteと識別する。完全な対象件数やselection結果を推測してはならない（MUST NOT）。0件とload failureを区別する。
render可能なrowにはSOURCE-EDIT-015のlossless projection / read-only reasonを使用する。Profile-independent検証によりselectionを安全に実行できない場合、raw閲覧とdiagnosticsは提供してもselected件数と理由はUnavailableとする。

### AUTHORING-OVERVIEW-003

Profile previewは同snapshotの保存済みprofileを解決し、Build Selection ownerを使用しなければならない（MUST）。初期はunfiltered、選択はこのProject sessionの明示state。消えたprofileはmissing stateになり、unfilteredへfallbackしない（MUST NOT）。
結果はTable別total/selected count、row別selected/excluded、matched include/exclude tags、include-emptyかinclude-unmatchedかを構造化して返す（MUST）。exclude優先を既存formulaから説明し、GUIで別selection判定を行わない。
defaultは全rowにmembership表示。selected-onlyは明示view filterとする。countはsearch/filter前datasetを表し、現在表示件数と区別する。Profile変更は通常File Viewのqueryやrowを変更しない。
dataset diagnosticsはshared validatorから取得し、unsupported/Draft Reference semanticsをfrontendへ実装してはならない（MUST NOT）。

## 受け入れ証拠

64-bit比較、ordinal string、invalid/null/同値のsort、search ORとfilter AND、unsupported operator、source不変を検証する。Overviewはsplit fileと同PK、partial parse、source途中変更、profile missing、exclude優先、selected-onlyとtotal countの区別を検証する。
