# Development State

Stage: decision-required
Candidate: none
Work base: 7e35caad818422bfd09c0bbd220f7e47453c38a0

## Active work

In progress: Standalone Web workspaceの仕様変更Draftと、共有Rust core / frontendによるread-only Browser Host checkpoint。
Remaining: Browser SaveのHuman decision、Table / Type multi-file recovery contract、source write実装、実ブラウザ操作、exact Candidate verificationとremote CI reconciliation。

## Blocking findings

None.

## Human decision needed

Browserのlocal file APIでは、外部processと競合するsource writeをatomic compare-and-swapで確定できない。保存直前のidentity照合と保存後の再読込で競合を検出し、曖昧な結果では自動再試行せずbufferを保持する残余raceを受容するか、Standalone Webのin-place Saveを延期するかを選ぶ。前者の場合、Table / Typeのmulti-file migrationにはdurable journal、rollback、再選択後のRecovery Requiredを伴う別の契約を策定する。Human decision後はspec review / approval、write実装、verificationへ自律継続する。
