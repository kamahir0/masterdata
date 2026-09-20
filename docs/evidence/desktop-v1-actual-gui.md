> **Historical evidence — not current authority.** Current behavior / statusはcanonical specifications、Development State、code / tests / Gitから確認する。このfileはactive Candidate / Objectiveから必要な場合だけ参照する。

# Desktop制作v1 actual Desktop GUI evidence

実行日: 2026-09-19（JST）

## 対象

- exact tested SHA: `d79e67753859025f33b6852eb712593136988177`
- GitHub Actions: https://github.com/kamahir0/masterdata/actions/runs/35440111781
- workflow: `.github/workflows/desktop-evidence.yml`
- scenario driver: `apps/gui/tests/desktop-e2e.mjs`

V01解消のため、Tauri command関数の直接呼出しではなく、production WebView assetsを含むactual Desktop binaryを起動し、Tauri WebDriver bridge + WebKitWebDriverから実際のGUI surfaceを操作した。

## 実行環境

- GitHub Actions `ubuntu-latest`
- Linux 6.17.0-1022-azure x86_64
- Rust `rustc 1.98.1 (48a229cea 2026-09-01)`
- Node.js `v22.23.2`
- .NET SDK `8.0.425`
- `tauri-driver 2.0.6`
- native driver: `/usr/bin/WebKitWebDriver`
- Desktop build: `npm --workspace @masterdata/gui run tauri -- build --no-bundle`
- display/session: Xvfb + D-Bus session

## 制作scenario

次の操作をactual Desktop GUIから順に実行し、すべてPASSした。

1. Create Project画面から新規Projectを作成。
2. New source artifact dialogからTable `item` を作成。
3. 同dialogを再度開き、Data documentを作成。
4. Data EditorのAdd Rowから`id = 1001`を入力し、Saveを実行。filesystem上のData document更新を確認。
5. Project SettingsからProfile `prod` とC# publish target `delivery` をbufferへ適用し、Save Settingsを実行。設定file更新を確認。
6. Delivery surfaceからBuild saved inputを実行し、`Build succeeded`を確認。
7. Publish preview作成後、destinationへ外部managed file + manifest変更を注入。
8. 旧Confirmを実行し、`E-PUBLISH-PREVIEW-STALE-DESTINATION`で拒否され、外部fileが未変更であることを確認。
9. fresh Publish previewを作成し直してConfirmし、`Publish completed`を確認。review済みremovalとmanifest更新をfilesystemで確認。

workflow logのscenario markers:

- `project-created-through-gui`
- `table-created-through-gui`
- `data-source-created-through-gui`
- `record-edited-and-saved-through-gui`
- `settings-profile-and-publish-target-saved-through-gui`
- `build-completed-through-gui`
- `external-publish-destination-change-injected`
- `stale-publish-rejected-without-mutation`
- `fresh-publish-completed-through-gui`
- final status: `pass`

Data documentはfilenameではなく`kind: data` / `table: item`のsemantic contentで特定している。これはWebDriverのcontrolled-input event差異をacceptance判定から外し、actual Desktop制作workflowの成立を検証するためである。

## 結論

Current Objectiveが要求する「Desktop実機制作scenario」は、actual Desktop window / WebView / Tauri bridge / filesystem / Build / Publishを通る形で完走した。stale Publish failure recoveryも同一scenario内で確認済み。
