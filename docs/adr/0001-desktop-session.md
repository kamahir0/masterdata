# ADR-0001: GUIが編集sessionを所有する

Status: Accepted

採用根拠: Desktop Authoringの別pass review後、2026-10-17のHuman委任に基づくAgent Decision。共有言語や旧application境界より、編集sessionと入力保持を優先する。

## 問題

旧プロジェクトでは一回の厳密なdomain commandとGUIの一操作を近づけたため、未完成の定義を作る途中にmodalや全体検証を要求し、エラー修正まで拒否しやすかった。frontendの見た目だけを変えても解消しない。

## Decision

Agent DecisionとしてElectron + TypeScript + React / Ant Designを採用する。rendererのimmutable Project draftが操作と履歴を所有する。意味検証はpureなmoduleとして分離し、backgroundで実行する。UIは同じ値入力componentを表・nestedで使い、native hostはopen / save / build / publishだけを提供する。

保存済みbaseと編集historyを分ける。dataはメモリ内の値として変更し、YAMLへのpatch / serializationはSaveまたは明示text表示時に行う。これにより入力やfolder開閉でdisk / 全量parseを待たない。snapshotは変更していないsource / recordを共有し、履歴は有限にする。

型変更時に入力を保持するため、draftを「その型で常にvalidな値」だけに制限しない。shapeと意味の診断は編集の受付から分ける。保存時の競合検出と安全性、Build時の厳密検証はそれぞれの境界で行う。

YAMLとMasterMemoryの利用目的は維持する。出力は.NETへ委譲し、既存backend全体を呼び出すCLI subprocessは作らない。Buildはデータのsnapshotを入力とし、GUI sessionの整合性や保存を強制しない。

## 比較した選択

- 旧Rust applicationの再利用: 厳密なcommandとdraftの変換を再導入しやすく、今回の編集モデルから独立して導けないため採用しない。
- Tauri + 新Rust model: 可能だが、メモリdraftを跨ぐmodelの複製とserialization boundaryを先に増やす。今回の規模ではTypeScriptの単一modelを優先する。
- 独自HTML UI: 統一された入力・keyboard・テーマ・motionを再実装するため採用しない。

## 根拠

[Desktop Authoring](../specs/desktop.md)、HumanのGUI単独・標準部品・豊かなmotion・legacy2を参考に自律導出する指示。Electronの[context isolation](https://www.electronjs.org/docs/latest/tutorial/context-isolation)と[限定したIPC](https://www.electronjs.org/docs/latest/tutorial/ipc)、Ant Designの[Table](https://ant.design/components/table/)と[theme / motion](https://ant.design/docs/react/customize-theme/)を確認した。
