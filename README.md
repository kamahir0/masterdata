# MasterData — Clean-room入力境界

human-readable YAMLをcanonical sourceとする、MasterMemory / Unity向けDesktop-first master-data authoring environment。

この`rewrite/clean-room` branchは正式contractと独立oracleだけを残した準備地点です。product implementationはありません。新implementationも開始していません。

読む入口は[Clean-room Handoff](docs/rewrite-preparation/clean-room-handoff.md)。product / UX / safety / performance / success oracleはそのreading orderに従ってください。

`main`とannotated tag `legacy-final`はReady legacy productionを保存します。tag/historyはforensic fallbackでありarchitecture authorityではありません。このbranchをmainへmergeしません。

境界検査: `python3 tools/check-clean-slate.py`（Python 3.11以上 / PyYAML）。旧runtime testsは意図的に退役済みであり、この検査の成功をproduct conformanceとは呼びません。
