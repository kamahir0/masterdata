# Browserのプロジェクト新規作成

Status: Applied

Basis: 2026-10-27 Human request「ブラウザ版MasterData（Web版）に『プロジェクト新規作成』機能を導入してください」。選んだフォルダ自体をProject rootとして初期化する最小権限モデル、`masterdata.toml`既存時の誤上書き防止、作成後の編集session開始とIndexedDB永続化、Desktop / CLI互換はHuman Requirement。以下の検証順序、名前規則、失敗時の後始末はAgent Decision。

## 課題

[Browser Authoring](../specs/browser.md)は、データ保護基盤を優先してBrowserでの新規作成を初期スコープ外とした（Agent Decision）。このためWelcomeの「新しく作る」は非表示、runtimeの`create`は常にエラーで、Browser利用者は先にCLI / Desktopでプロジェクトを作る必要があった。

## 提案

1. **操作フロー**: 対応Browserでは、Welcomeに「新しく作る」を表示する（非対応Browserでは「プロジェクトを開く」と同様に無効表示し、対応環境を案内する）。押すとプロジェクト名入力modalを表示し、「保存先を選択」でDirectory Pickerを開く。pickerはユーザー操作の中で、ファイルI/Oより前に呼ぶ。
2. **最小権限**: Desktopのように親フォルダの下へ子フォルダを作らない。**利用者が選んだフォルダ自体をProject rootとして初期化**する。親フォルダへの権限は要求しない。modalにその旨を表示する。
3. **作成内容**: root直下に`masterdata.toml`（Desktop / CLI `init`と同じ既定設定で`name`に入力名のtrimした値）と`sources/`を作る。それ以外のfileは作らず、既存のfileや`sources/`内の内容を変更しない。選んだfolderが空でなくてもよい（CLI `init`と同じ）。
4. **事前検証と安全性**:
   - project名は、trim後に空でなく、`.` / `..`でなく、Desktopの新規作成と同じ禁止文字（`< > : " / \ | ? *`と制御文字）を含まないこと。不正ならpickerを開く前に理由を示し、modalと入力を保持する。
   - 選択folderに`masterdata.toml`が既にある場合は何も書かず、「既存のプロジェクトです。『プロジェクトを開く』から開いてください」と案内する。`masterdata.toml`が通常fileでなくfolderの場合、`sources`が通常fileの場合も何も書かず理由を示す。
   - 中断した保存の復旧情報（`.masterdata/browser-transactions/`）が残るfolderは、通常のオープンと同じ復旧導線に従い、新規作成しない。
   - 書込み前にreadwrite権限を確認する。拒否・取消（`AbortError`）・picker例外では何も作らない。取消は無言でmodalに留まり、拒否は理由を示す。
5. **書込み順序と失敗時**: `sources/`を作り、最後に`masterdata.toml`を書く（設定fileの存在がProjectの成立を表すため）。作成後に通常のオープンと同じ読込検証を行い、失敗した場合は今回書いた`masterdata.toml`だけを削除して理由を示す。書込み・読込検証の失敗でも既存のfileを削除しない。処理は同一originのfilesystem lockで直列化し、保存・読込・復旧処理中は開始しない。
6. **セッションと永続化**: 成功後はそのまま編集sessionを開始する。DirectoryHandleをIndexedDBの直近プロジェクトへ保存し、次回起動時の自動復元またはワンクリック再開の対象にする（[ハンドル永続化](0023-browser-persistent-project-handle.md)と同じ）。IndexedDB失敗でもProjectの使用を妨げない。
7. **非変更**: Desktopの新規作成（親folder選択、子folder作成）、CLI `init`、`masterdata.toml`とYAMLの形式、sample作成（引き続きBrowser非対象）、Build / Publish非表示は変更しない。

## Acceptance / validation

1. 空folderを選ぶと`masterdata.toml`（指定名、既定profile / publish）と空の`sources/`だけが作られ、編集sessionが開始し、未編集なら保存済み表示になる。作成物はRust CLIの`validate` / Desktopで開ける。
2. 既存`masterdata.toml`のfolderでは書込みが0件で、既存Projectを開く案内が出る。不正な名前はpickerを呼ばない。取消・拒否では何も作らず、modalへ戻る。
3. 既存fileのあるfolderでも、それらを変更せず`masterdata.toml`と`sources/`だけを追加する。読込検証に失敗した場合は今回の`masterdata.toml`だけを撤去する。
4. 作成後にIndexedDBの直近プロジェクトが更新され、再起動で自動復元またはワンクリック再開できる。
5. 非対応Browserでは「新しく作る」が無効で理由が分かる。Desktop版のWelcomeと新規作成は変わらない。
6. focused unit tests（File System Access mock）とChromiumのUI回帰（実のOPFS directory）が通り、Desktop回帰が維持される。実Chromeの実フォルダでの手動検証と、Edgeの未検証は[Development State](../execution-state.md)で区別する。

## Canonical owner

- [Browser Authoring](../specs/browser.md)（目的・対象外、WEB-001、WEB-005、Acceptance）

## Review

別passでchallenge。
- Human intentへの適合: 選択folder自体の初期化、`masterdata.toml`既存時の保護、作成後のsession開始とIndexedDB保存、Desktop / CLI互換をすべて取り込んでいる。
- 権限境界: 要求する権限は利用者が選んだfolder一つで、既存のBrowser I/O境界（root直下の`masterdata.toml`、`sources/`）を拡張しない。親folder権限、新しい外部送信、公開はなし。
- 状態と失敗: 名前不正、既存Project、復旧情報あり、権限拒否、取消、書込み失敗、読込検証失敗の各結果と、データが残る範囲が具体的。`masterdata.toml`の確認と書込みの間の他プロセスによる作成は、他Browser APIでOSレベルに排他できないため完全には防げない。この限界は既存の外部writerの扱いと同じで、WEB-004の主張範囲を変えない。
- 検証可能性: mockによる単体試験と実Browserの回帰で、書込み0件、作成内容、永続化を観察できる。
- Blocking Issues: None
- Non-blocking Issues: 空でないfolderの選択を許すため、利用者が誤って大きな既存folderを選んでも`masterdata.toml`と`sources/`以外は変更されない旨をmodalで明示する。
- Questions: None
- Approved as Proposed: Yes
- Autonomous approval eligible: Yes（Human明示依頼のObjective内。permission modelは既存の利用者選択folderの範囲に収まり、境界を拡張しない最小権限のためHuman gateなし）
- Human gate: None

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-27 Human requestによるBrowserプロジェクト新規作成の仕様策定・実装・検証・通常Git deliveryの委任。review passの判定後、orchestrationが承認。
