# Historical Evidence

`docs/evidence/**` は過去candidateのreview、manual verification、performance measurement等を保持するhistorical evidence areaである。

これらはcurrent specification、Current Objective、Development State、current implementation realityのauthorityではない。fresh sessionでは、active Candidate / Current Objective / canonical specから明示参照された場合だけ必要なfileを読む。

過去のfinding、benchmark、CI resultをcurrent defect / current performanceとして無検証で再利用してはならない。current truthはcode / tests / Git / current CIから確認する。

evidenceを残す目的はhistorical traceabilityであり、同じbehaviorがfocused regression testやcanonical ownerへ十分移った後に新しいsummary documentを重ねて作ることではない。
