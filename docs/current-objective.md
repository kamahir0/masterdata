# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Git-native Collaboration & Automationをproduction-readyにし、YAML Source of Truthとshared semantic analysisをGit working tree / review workflowへ接続して、人間とAIが同じchange evidenceから安全にreview・commit・collaborateできる状態へ到達する。**

## Completion slices

### Git-aware review

- Project rootのGit repository状態をshared application boundaryからread-onlyに取得し、branch / HEAD / dirty / untracked / conflict / detached / ahead-behind等をfilesystem/YAML semanticsと混同せず表現する。
- raw text diffだけでなく、MasterData semantic change、Migration/Compatibility impact、source provenanceをreview surfaceへcompositionできるようにする。
- dirty editor buffer、saved workspace、Git index / HEAD snapshotの違いを明示し、未保存bufferをGit changeとして偽装しない。

### Safe local workflow

- Human decisionでlocal mutationをscopeへ含める場合、stage / unstage / commitをexplicit operationとして扱い、対象path / diff / commit messageをreviewしてから実行する。
- unrelated user changesを自動stage / discard / stash / resetしない。
- conflict / merge / rebase / detached / diverged等のstateをfail closedで扱い、history rewriteを通常automationへ含めない。
- AI/Human双方が同じstructured change setを参照できるようにする。

### Remote collaboration boundary

- Human decisionでremote mutationをscopeへ含める場合だけ、push / branch publication / Pull Request等をcredential/trust boundary付きの別phaseとして設計する。
- local commit successをremote publish successとして扱わない。
- push / PR作成をhidden side effectやBuild/Publishの付随動作にしない。

### Verification

- temp Git repositoriesによるstatus / diff / conflict / local mutation safety evidenceを作る。
- existing source-preserving editor、Migration、Compatibility、Build / Publish semanticsをregressさせない。
- fresh review、repository checks、exact Candidate、required remote CI reconciliationまで完了する。

## Human-gated scope boundary

Git-native v1でproductがどこまでrepository mutationを所有するかは仕様変更0029でHuman decisionを受けて確定する。

推奨は **Option B: read-only Git awareness + explicit local stage/commit**。remote push / PR mutationはv1非対象とする。

この方向ならProduct VisionのGit diff/review/automationを大きく前進させつつ、credential / remote authority / external irreversible effectを別Objectiveへ隔離できる。

## Explicit non-scope

Human decisionで選択されない限り、以下はv1へ含めない。

- force push、rebase、reset --hard、history rewrite、automatic stash。
- merge conflictの自動解決。
- generated artifact / Unity package artifactのautomatic commit。
- release tag / GitHub Release / package registry publish。
- credential storage / token management。
- repository hosting provider固有のpermission model。
- arbitrary Git hook installation。
- cloud collaboration serviceやWeb product surfaceの再導入。

## Audit

2026-09-22 JST、Production Delivery & Unity Integration完了後、Humanは次の大きなObjectiveへ進むことを選択した。Authoring system RFCでDeferredとなっていたGit automationとProduct VisionのGit diff/review/automation方向を次priorityとしてGit-native Collaboration & Automationを開始する。コード編集を伴う実装はimplementation agentへ指示書で委譲する既存運用を継続する。
