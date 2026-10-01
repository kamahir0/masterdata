# ADR 0008: Interactive workspace read session

Status: Accepted

## 背景（Context）

[baseline](../evidence/interactive-navigation.md)では通常source切替で同じprojectの8 sourceを2回、schema redirectで3回parseし、再訪にもfull parseが残る。statelessなcommandごとのproject再構築はview切替のlifetimeに適さない。UI threadから同期readを呼ぶ構成はread durationをdesktop repaintにも結び付ける。

## 決定（Decision）

Desktopのshared Application boundaryは、project bindingに属するlong-lived workspace read sessionを所有する。source inventory、exact loaded source identities、parse result/diagnostic、Table context・source ownership・authoring dependency indexを一つのimmutable generationへまとめる。同じgenerationからTable/Data/Type viewを導出し、同じYAMLをview別に再parseしない。CLIのone-shot readは同じshared derivationをshort-livedに使用し、semanticsをforkしない。

read cacheはYAML source authorityではない。navigationでは対象sourceとauthoring dependencyのcurrent bytesを確認し、変更時は古いparse resultをcurrentから外す。external inventory変更はcoalesced background reconciliationへrouteする。config/binding変更は黙って旧bindingをeditableにせず、明示的なreloadへrouteする。parse不能・削除されたsourceの旧generationをcurrentとして返さず、unrelated parseable sourceは共有Coreのcapabilityに従って使用できる。

read workはnative UI threadとmigration plan lockから分離する。obsolete selectionはprojection前にcoalesce/cancelし、開始済みでcancel不能なworkもgeneration/tokenでresultをdiscardする。長いparse / projection / validation中にworkspace publication lockを保持しない。

local authoring shapeを得たviewはproject-wide diagnostics完了より先にusableにできる。validationはcaptured immutable generationに対してbackgroundで行い、結果は同じgenerationだけへ適用する。dirty schema / record overlayはbase read generationと別のauthoring ownershipに残す。

Save / structural operationのactual source identity、expected identity、path safety、Conflict / Recoveryとsource-preserving commitは既存native write contractを使用する。cached identityの一致をcommit authorizationへ変換しない。新read path成立後はGUIのper-command full-load APIとduplicate orchestrationを除去し、旧full reload fallbackを常設しない。

## 結果（Consequences）

project loadのCPU/memory costをsession lifetimeへ移し、通常navigationから繰り返すparse/validationを除く。memoryはproject終了/binding変更で解放する。freshness check、generation swap、dirty overlay compositionとdiagnostics ownershipが明示的に必要になる。一般的incremental compilerやwatcher systemへ拡張しない。

## 代替案（Alternatives）

frontend memoizationやfull rebuildのbackground化だけではduplicate parseとownership分裂が残る。per-command cacheを重ねる方式もinvalidationを複数ownerへ分散させるため採用しない。payload paginationはbaselineのserialization costから支配的ではなく、今回は先行導入しない。
