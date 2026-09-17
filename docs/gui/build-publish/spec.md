# GUI仕様: Build / Publish

Status: Approved

Build / Publish surfaceは保存済みProject inputからcanonical Buildを実行し、receipt-valid artifact setを既存Publish contractに従って外部targetへ配置するDesktop workflowを定義する。domain contractは[Build pipeline](../../specs/build-pipeline.md)と[Build Selection](../../specs/build-selection.md)が所有する。適用記録は[仕様変更0018](../../spec-changes/0018-desktop-build-delivery.md)を参照する。

## 規範要件

### GUI-DELIVERY-001

Build / Publish surfaceはProject、Profile、dirty source数/config有無、Build/native capabilityと利用不可理由を表示し、Build / Publish last successful artifacts / Build and Publishを別actionにしなければならない（MUST）。初期Profileはunfilteredで、Overviewと同じ明示Project-session selectionを共有する。missing profileは選択がmissingと分かる状態でBuildを拒否し、unfilteredへfallbackしない。
Profile選択が無効でも、current configがload可能ならPublish-onlyのreceipt authorityを否定しない（MUST）。Publish-onlyはProfile selectionではなくcanonical setを対象とする。

### GUI-DELIVERY-002

Buildはdirty中も実行可能で、保存済みsource/configだけを使う表示と独立したSave All actionを提供しなければならない（MUST）。SaveをBuild actionの隠れた前処理にしない。
stateはidle、running、succeeded、failedを区別し、進行中operationのProject/captured Profileを表示する。結果にはstructured diagnostics、artifact root、receipt検証結果を保持する（MUST）。失敗時の既存artifact残存を今回Build成功と表示してはならない（MUST NOT）。errorから対応sourceへ移動する際はOverviewのsnapshot/provenance確認を通す。

### GUI-DELIVERY-003

Publish-onlyはlast successful canonical artifact setをreceipt authorityとして扱い、current YAMLのdirty/validityやcurrent Profile selectionをPublish eligibilityへ混ぜてはならない（MUST NOT）。current configのtargetを読み、receipt validationとall-target preflightを経たpreviewを表示してからConfirmする。
valid receiptで0 targetは既存contractどおりsuccessful no-opと表示し、配置成功と誤認させない。retryは全targetに対する新previewから開始し、失敗targetだけを勝手に再試行しない（MUST NOT）。結果不明ならautomatic retryをせずactual artifact/destination確認へ誘導する。
Publish-onlyはcurrent YAMLをparse/hash/validate/compareせず、source freshnessを常に「この操作では未確認」と表示する（MUST）。receiptからBuild時Profileを推測せず、過去sessionのProfileをcurrent artifactのprovenanceとして表示しない。Unity compile / 動作確認の成功を含意しない。

### GUI-DELIVERY-004

Build and Publishは最初に「保存済みinputでBuildし、成功artifactのPublish previewへ進む」操作であることを明示しなければならない（MUST）。Build成功後にPublish previewで対象と影響を確認してからPublishを開始する。二段階のGUI導線であり、単独Buildの自動Publishではない。
Build失敗ならPublish未実行。Build成功後にpreview/Publishが失敗またはCancelでもsuccessful canonical setをrollbackしない（MUST）。結果はBuild success / Publish not-run-or-failedを分ける。
Build開始からpreviewまでにsaved config/targetが変われば変更を表示し、新しいpreviewの確認を要求する（MUST）。同じapp内の別Buildでartifactが入れ替わった場合も古いBuild結果をそのままPublish対象にしない。

### GUI-DELIVERY-005

同一Projectの同一app sessionではBuild、Publish、Migration Apply、config Saveを同時実行せず、理由付きbusy stateで二重開始を防がなければならない（MUST）。通常source SaveはBuildPlan確定まで待機し、その後は可能とする。待機中actionを暗黙queueして後でmutationせず、利用者の再操作を要する。
buffer編集、read-only navigation、Diff、Problemsは継続可能とする。Project切替 / closeは実行中mutationが確定するまで通常操作として完了させない（MUST）。OS強制終了へのatomicityは保証しない。
Migration Recovery Requiredではconfig/source mutationとBuild / Build and Publishを停止する。Publish-onlyはsourceに依存しないため、configをloadできreceiptとpreflightがvalidなら既存契約に従って利用可能とする（MUST）。source recoveryが完了したという表示にはしない。

### GUI-DELIVERY-006

Problemsはcurrent buffer、saved validation、Build resultをsnapshotとProfile付きで区別しなければならない（MUST）。古いdiagnosticをcurrentへ置換しない。sourceにmapできない問題も保持する。
operation開始・完了、partial failure、not_attempted、dirty-input除外、disabled reasonは文字とassistive semanticsで提示し、keyboardでaction、preview、result、source navigationへ到達できなければならない（MUST）。Confirm / Cancel後のfocusは開始actionへ戻す。progressの細かな段階数やpercentを推測しない。

## 受け入れ証拠

0 target、全target success、preflight全not_attempted、target部分失敗、unknown result、Build成功後Cancel / Publish failure、repeated click、busy中config Save、Migration recoveryでBuild不可/Publish-only可、Problems source移動のsnapshot不一致を検証する。
