# TOMLプロジェクトファイルと開く対象

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-20 Human Decision: project名は設定内とroot名で表せるためproject file名を固定する。「開く」はfolderではなくprojectを象徴するfileを選ぶ。JSONより設定項目追加時のcomma差分が少ないTOMLを好み、「それで修正」と実装を指示。データ本体はYAMLのまま。

## Behavior / Agent Decision

- `masterdata.toml`を唯一のproject設定にする。現在のname / profiles / publish構造をTOMLで読み書きし、fileがあるfolderをrootとする。`sources/`のYAMLと内部`.masterdata/`は維持する。旧YAML設定や旧CLI設定構造へのfallback / importは廃止し、ユーザーの既存fileを自動変換・削除しない（development phase）。
- 「開く」はTOML file用native pickerを表示し、固定名`masterdata.toml`を選ぶ。任意のTOML、YAML、folderをprojectとして誤認しない。cancelは現projectを保持し、読込失敗はfileと理由を示して現draftを保持する。既存の未保存切替保護を維持する。internal startup pathも同じproject fileを渡す。
- 新規作成は保存先folderを選び、新しいroot内へ`masterdata.toml`と`sources/`を作る。表示名は設定内nameを使い、root名が後で変わっても設定の名前を強制同期しない。同梱sampleもTOMLにする。
- 設定のsyntax / 既知項目のshapeが不正なら設定を黙ってdefaultに置き換えず、読込を失敗させる。nameは空でない文字列を必須とし、省略したprofiles / publish設定にはdefaultを補う。変更していない設定textを保存の副作用で書き直さない。設定を変更して保存するときはアプリ管理のblock TOMLへ整形する（任意の手編集comment / layoutのroundtrip保持は今回対象外）。未知の設定項目の値は保持する。設定の外部変更衝突は保存前に検出し、sourceを含めて一括で書込みを中止する。設定の保存も既存transaction保護の対象とする。Publishでproject fileを上書きできない。

## Acceptance / validation

sampleをproject fileでopen→name / profile / publish先変更→全保存→TOML再parse / reopen一致、source保存だけでは設定textを変更しない、作成したfileを再open、folder / 異なるfile名 / 不正syntax・shape拒否、取消とfailed openで現在draft保持、外部設定変更でsourceも書かない、Publish入力保護をfocused testsとnative pickerの実画面で確認する。OS file association / 自動移行は対象外。

## Review

別passでfileとfolderの区別、誤ったTOMLの受理、defaultによる設定喪失、cancel / failed open時の旧draft、外部変更の一括preflight、旧資料とのboundaryをchallenge reviewした。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（Humanが選択した形式と開く操作の変更内、実作業fileの変換はしない）。Human gate: None。
