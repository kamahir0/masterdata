# .NET builder boundary

This project is intentionally a tiny, dependency-free .NET executable used by
`masterdata-dotnet` for a real bridge smoke test. It verifies that Rust can
invoke a repository-owned .NET project and read its exit status/output.

It does **not** implement MasterMemory binary generation. The eventual builder
will own the MasterMemory v3 Source Generator, C# compilation, binary build,
and binary validation. Rust must remain an adapter and must not reimplement
those internals.

Manual smoke test:

```bash
dotnet run --project dotnet/builder/masterdata-builder.csproj -- --self-test
```

