# ADR 0007: record source valueとschema解釈を分離する

Status: Accepted

## 背景（Context）

従来はYAML parserがrecord scalarをBool / Number / Stringへ分類してからType Systemへ渡していた。そのため、field typeを変更しても同じsource textを別の意味で読むことができず、通常のheader操作が全record検証付きの即時Migrationになっていた。GUIだけの再解釈ではCLI ValidateやBuildと意味が分岐する。

## 決定（Decision）

shared Coreはrecord valueのsource representation（decoded scalar text、Null、Sequence、Mappingとsource provenance）と、resolved schemaに基づくtyped value / diagnosticを別の層として扱う。quote/styleはsource syntaxとして保持し、MasterData typeのauthorityにしない。raw sourceは保存の正本であり、解釈結果はschemaやdraftに応じて再計算する。

GUI、CLI Validate、Build、Migrationは同じCore interpreterを使用する。Save candidateの安全性とsemantic validityは別に判断し、Buildはvalidなresolved modelだけをloweringする。parser libraryと内部ASTの具体形は固定しない。observable scalar grammarとSave/Build境界はcanonical specificationsが所有する。

## 結果（Consequences）

document loading、Type System、Table resolution、authoring snapshot、source-preserving patch、.NET loweringの境界でsource valueとtyped valueを混同しない設計が必要になる。semantic-invalidなscalarもsourceとして保持し、診断と修復操作に渡せる。既存のvalid sourceは書き換えず、新たにvalidになるsourceについてCLI/GUI/Buildの結果が同時に変わる。

Human approval: specification change 0042に対する2026-09-27の明示的承認。
