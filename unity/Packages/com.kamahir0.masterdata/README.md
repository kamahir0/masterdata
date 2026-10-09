# MasterData Unity package

Unity Package Managerの「Add package from disk」でこのdirectoryの`package.json`を指定する。API baselineはUnity 2022.3。実Editor / Playerでの検証はportable .NET検証とは別に行う。

Build / Publish先は既存target設定を使う。generated C#を`Assets/MasterData/Generated`、binaryを`Assets/StreamingAssets/masterdata.bytes`へ配布できる。consumer側でMasterMemory 3.0.4 / MessagePack 3.1.3を導入する。

```csharp
using MasterData.UnityRuntime;

// Explicit bytes. The caller retains the returned database.
var result = SavedDatabaseLoader.LoadBytes(bytes,
    data => new Masterdata.Generated.MemoryDatabase(data));
if (!result.Succeeded)
    UnityEngine.Debug.LogError(result.Error.Code + ": " + result.Error.Message);
else
    database = result.Database;

// Inside a caller-owned coroutine. Reload uses another explicit call.
yield return StreamingAssetsDatabase.Load("masterdata.bytes",
    data => new Masterdata.Generated.MemoryDatabase(data),
    loaded => {
        if (loaded.Succeeded) database = loaded.Database;
        else UnityEngine.Debug.LogError(loaded.Error.Code + ": " + loaded.Error.Message);
    });
```

Editorの`Window > MasterData > Delivery`でexact generated C# directoryとbinary fileを指定し、`Observe selected artifacts`を実行する。compiler callbacksをまだ観測していなければcompileは`unknown`。Publish、import、compile、runtime loadの結果はそれぞれ独立して確認する。

verification:

```sh
python verification/check_unity_package.py
dotnet build verification/unity-compatibility/UnityCompatibility.csproj
dotnet run --project verification/unity-boundary/UnityBoundary.csproj
cargo test -p masterdata-engine --features native-consumer --test build_consumer unity_caller
```

Unity Test Frameworkではpackageを`testables`へ追加し、`MasterData.UnityTests` EditMode assemblyを実行する。actual Unity import / compile / attached window / StreamingAssets readはこの段階で検証する。
