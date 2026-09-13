from pathlib import Path

path = Path("crates/xtask/tests/execution_state.rs")
text = path.read_text()
old = '        "HumanへStage名だけを返してlaneを推測させてはならない",'
new = '        "HumanへStage名だけ、またはlaneだけを返して次がimplementationかnon-implementationかを推測させてはならない",'
if text.count(old) != 1:
    raise SystemExit(f"expected exactly one legacy lane assertion, got {text.count(old)}")
path.write_text(text.replace(old, new, 1))
