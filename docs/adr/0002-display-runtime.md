# 表示用JavaScriptの実行環境

Status: Accepted

Browserは静的アセットからReactを起動し、Desktopは同じGridをTauri WebViewで表示する。両方のscript-srcはselfに制限され、Browserの編集・保存はネットワークサービスを使わない。通常列のYAML整数をBigIntとして読込む。VMの選定ではこの三点を優先した。

| 言語・候補 | この製品での判断 |
| --- | --- |
| JavaScript / QuickJS | 両GUIで同じソース、BigInt・標準配列操作が使え、ヒープ・スタック上限と割込を設定できる。asm.js配布物を遅延ロードすればWasmの許可も不要。Worker自体はブラウザAPIを持つため、プロジェクトのコードはその内側のVMにだけ渡す。[QuickJSバインディング](https://github.com/justjake/quickjs-emscripten) |
| TypeScript | 利用者が型を使える利点はあるが、配布済みGUIで編集したソースを変換し、ソース位置を戻す工程が加わる。出力JSの隔離は別途必要。[TypeScript Handbook](https://www.typescriptlang.org/docs/handbook/2/basic-types.html) |
| Python / Pyodide | 整数や集計を記述しやすい。Pythonランタイムの配布、JSとの型変換、標準のJS相互運用を制限する追加設計が発生する。[Pyodide FFI](https://pyodide.org/en/stable/usage/api/python-api/ffi.html) |
| Lua | WasmoonはLua VMをWasmで配布する。Fengariは純JSで比較的小さいが整数型が32bit。GUIのデータ型・配布制約との調整を要する。[Wasmoon](https://github.com/ceifa/wasmoon)・[Fengari](https://github.com/fengari-lua/fengari) |
| Luau | sandbox・interruptの設計を持つ。ホスト側で配布・割込・メモリ制限を構成し、精密整数の受渡しを別途設計する必要がある。[Luau Sandbox](https://luau.org/sandbox/) |

QuickJSのasm.js同期版を採用し、専用Workerに同梱する。TS実装、Rust IPC、C#生成のどれにも新しいスクリプト言語やホスト権限を追加せず、JSを一つのVMで評価できる。ホスト側のevalや動的Function、CDNからのコード取得を不要にする。実行の開始条件は[Grid評価](../specs/display-evaluation.md)の判断に従う。

2026-11-11にmacOSで隔離した一時プロジェクトを使用。quickjs-emscripten-coreと@jitl/quickjs-asmjs-mjs-release-syncはいずれも0.32.0。専用Worker bundleはminify後1,027,064 bytes、gzipで332,762 bytes。localhostのHTTP CSPをscript-src self、worker-src self、connect-src noneとしてPlaywright ChromiumとWebKitで確認した。両方でulong最大値を十進文字列として保持し、20%、30%、50%を表示した。fetch、process、document、postMessage、indexedDBはゲスト側でundefined。50msの期限でループを中断し、12MiBのゲストヒープを上回る文字列処理をメモリ不足で停止し、続く42の評価が成功した。この数値はVM選定用の測定で、製品Gridの負荷測定は別途行う。
