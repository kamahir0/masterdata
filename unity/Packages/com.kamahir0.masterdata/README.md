# MasterData Unity Integration

This repository package is a thin Unity delivery boundary. Configure the
MasterData project with its existing `publish.targets`, for example:

```toml
[[publish.targets]]
kind = "csharp"
path = "../unity/Assets/MasterData/Generated"

[[publish.targets]]
kind = "binary"
path = "../unity/Assets/StreamingAssets/masterdata.bytes"
```

The generic publisher owns generated C# files, the explicit binary, and its
publish manifest. Unity owns `.meta` files and AssetDatabase lifecycle. The
package does not parse MasterData YAML, infer publish paths, generate GUIDs, or
delete orphan metadata.

The runtime API delegates database construction to the generated MasterMemory
type supplied by the Unity project. With the repository's pinned
MasterMemory 3.0.4 / MessagePack 3.1.3 setup, a consumer can compose:

```csharp
var database = await MasterDataRuntime.LoadStreamingAssetsAsync(
    "masterdata.bytes",
    bytes => new MemoryDatabase(bytes),
    cancellationToken);
```

The caller owns `database` and explicitly repeats the load to create a new
instance. The Editor menu `MasterData/Delivery Status` observes exact selected
asset paths and keeps Publish, import, compile, and runtime status separate.
