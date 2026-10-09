#!/usr/bin/env python3
"""UPM layout and dependency boundaries; not Unity Editor/Player verification."""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = ROOT / "unity/Packages/com.kamahir0.masterdata"
EXPECTED = {
    "package.json", "README.md",
    "Runtime/MasterData.UnityRuntime.asmdef", "Runtime/SavedDatabaseLoader.cs",
    "Runtime/StreamingAssetsLocation.cs", "Runtime/StreamingAssetsDatabase.cs",
    "Editor/MasterData.UnityEditorIntegration.asmdef", "Editor/ArtifactSnapshot.cs",
    "Editor/ArtifactObservation.cs", "Editor/DeliveryObservationWindow.cs",
    "Tests/Editor/MasterData.UnityTests.asmdef", "Tests/Editor/DeliveryTests.cs",
}
DIAGNOSTICS = {
    "MASTERDATA-UNITY-MISSING", "MASTERDATA-UNITY-EMPTY", "MASTERDATA-UNITY-READ",
    "MASTERDATA-UNITY-CANCELLED", "MASTERDATA-UNITY-FACTORY", "MASTERDATA-UNITY-SCOPE",
    "MASTERDATA-UNITY-CSHARP-MISSING", "MASTERDATA-UNITY-BINARY-MISSING", "MASTERDATA-UNITY-BINARY-EMPTY",
    "MASTERDATA-UNITY-IMPORT-PENDING", "MASTERDATA-UNITY-IMPORT-UNKNOWN",
    "MASTERDATA-UNITY-COMPILE-PENDING", "MASTERDATA-UNITY-COMPILE-UNKNOWN", "MASTERDATA-UNITY-COMPILE-FAILED",
    "MASTERDATA-UNITY-COMPILER-MESSAGE", "MASTERDATA-UNITY-METADATA-PRESERVED", "MASTERDATA-UNITY-OBSERVATION-READ",
}


def check():
    files = {path.relative_to(PACKAGE).as_posix() for path in PACKAGE.rglob("*") if path.is_file()}
    assert {name for name in files if not name.endswith(".meta")} == EXPECTED, "Unexpected/missing package source layout"
    assert all(name[:-5] in EXPECTED or (PACKAGE / name[:-5]).is_dir() for name in files if name.endswith(".meta")), "Unowned package metadata"
    assert {path.name for path in (ROOT / "unity").iterdir()} == {"Packages"}, "Unexpected Unity workspace root"
    assert {path.name for path in (ROOT / "unity/Packages").iterdir()} == {"com.kamahir0.masterdata"}, "Unexpected Unity package"
    manifest = json.loads((PACKAGE / "package.json").read_text())
    assert manifest["name"] == "com.kamahir0.masterdata" and re.fullmatch(r"\d+\.\d+\.\d+", manifest["version"])
    assert manifest["unity"] == "2022.3" and not manifest.get("dependencies"), "Hidden package dependency"
    runtime = json.loads((PACKAGE / "Runtime/MasterData.UnityRuntime.asmdef").read_text())
    editor = json.loads((PACKAGE / "Editor/MasterData.UnityEditorIntegration.asmdef").read_text())
    tests = json.loads((PACKAGE / "Tests/Editor/MasterData.UnityTests.asmdef").read_text())
    assert runtime["references"] == [] and not runtime.get("includePlatforms"), "Runtime depends on Editor/consumer assembly"
    assert editor["includePlatforms"] == ["Editor"] and editor["references"] == [runtime["name"]]
    assert tests["includePlatforms"] == ["Editor"] and tests["references"] == [runtime["name"], editor["name"]]
    assert tests["optionalUnityReferences"] == ["TestAssemblies"] and tests["autoReferenced"] is False
    sources = {}
    for name in sorted(EXPECTED):
        if not name.endswith(".cs"):
            continue
        source = (PACKAGE / name).read_text()
        # Comments may cite canonical semantics; only executable dependency
        # boundaries are checked here. Behavioral evidence uses actual callers.
        code = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)
        sources[name] = code
        if name.startswith("Runtime/"):
            assert not re.search(r"\bUnityEditor\b|\b(?:MasterMemory|MessagePack|YamlDotNet)\b", code), name
        if not name.startswith("Tests/"):
            assert not re.search(r"\b(?:YamlDotNet|MasterMemory|MessagePack)\b|\bProcess\s*\.", code), name
            assert not re.search(r"(?:File|Directory)\s*\.\s*(?:Write\w*|Delete|Move|Copy|Create\w*)\s*\(", code), "Observation/runtime wrote files: " + name
    found = set(re.findall(r'MASTERDATA-UNITY-[A-Z-]+', "\n".join(sources.values())))
    assert found == DIAGNOSTICS, ("Diagnostic vocabulary", found ^ DIAGNOSTICS)
    streaming = sources["Runtime/StreamingAssetsDatabase.cs"]
    assert "UnityWebRequest.Get" in streaming and "Application.streamingAssetsPath" in streaming and "request.Abort()" in streaming
    observer = sources["Editor/ArtifactObservation.cs"]
    assert "AssetDatabase.LoadAssetAtPath" in observer and "CompilationPipeline.assemblyCompilationFinished" in observer
    assert "CompilationPipeline.compilationStarted" in observer and "CompilationPipeline.compilationFinished" in observer
    window = sources["Editor/DeliveryObservationWindow.cs"]
    assert "new TextField" in window and "new HelpBox" in window and "Runtime load: not observed" in window
    print(json.dumps({"unityPackageStatic": "PASS", "sourceFiles": len(sources), "diagnosticCodes": len(found),
                      "unityEditorCompile": "not_observed", "unityPlayer": "not_observed"}, indent=2))


if __name__ == "__main__":
    check()
