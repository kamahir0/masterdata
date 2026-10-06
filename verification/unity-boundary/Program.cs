using System;
using System.IO;
using System.Linq;
using System.Threading;
using MasterData.UnityRuntime;
using MasterData.UnityEditorIntegration;

internal static class Program
{
    private sealed class Database { public byte[] Bytes; }
    private static void Check(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Error(LoadResult<Database> result, string code)
    { Check(!result.Succeeded && result.Database == null && result.Error.Code == "MASTERDATA-UNITY-" + code, code); }
    private static void Main()
    {
        var bytes = new byte[] { 1, 255, 3 };
        var calls = 0;
        Func<byte[], Database> factory = value => { calls++; return new Database { Bytes = value }; };
        var first = SavedDatabaseLoader.LoadBytes(bytes, factory);
        var second = SavedDatabaseLoader.LoadBytes(bytes, factory);
        Check(first.Succeeded && second.Succeeded && calls == 2 && ReferenceEquals(first.Database.Bytes, bytes)
            && !ReferenceEquals(first.Database, second.Database), "Factory/caller ownership changed");
        Error(SavedDatabaseLoader.LoadBytes<Database>(null, factory), "MISSING");
        Error(SavedDatabaseLoader.LoadBytes(Array.Empty<byte>(), factory), "EMPTY");
        Error(SavedDatabaseLoader.LoadBytes<Database>(bytes, null), "FACTORY");
        Error(SavedDatabaseLoader.LoadBytes<Database>(bytes, _ => null), "FACTORY");
        var cause = new InvalidOperationException("caller failure");
        var failed = SavedDatabaseLoader.LoadBytes<Database>(bytes, _ => throw cause);
        Error(failed, "FACTORY"); Check(ReferenceEquals(failed.Error.Cause, cause), "Factory cause lost");
        using (var cancellation = new CancellationTokenSource())
        {
            cancellation.Cancel();
            Error(SavedDatabaseLoader.LoadBytes(bytes, factory, cancellation.Token), "CANCELLED");
            Check(calls == 2, "Cancelled or missing load invoked factory");
        }
        using (var cancellation = new CancellationTokenSource())
            Error(SavedDatabaseLoader.LoadBytes(bytes, value => { cancellation.Cancel(); return factory(value); }, cancellation.Token), "CANCELLED");
        Error(SavedDatabaseLoader.LoadBytes<Database>(bytes, _ => throw new OperationCanceledException()), "CANCELLED");
        Console.WriteLine("PASS caller-owned bytes/factory/reload and missing/empty/cancellation/factory errors");

        var root = Path.Combine(Path.GetTempPath(), "masterdata-unity-boundary-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var url = StreamingAssetsLocation.Resolve(root, "nested/file #1%?.bytes");
            Check(new Uri(url).IsFile && new Uri(url).LocalPath == Path.Combine(root, "nested", "file #1%?.bytes"), "Literal path escaping");
            Check(StreamingAssetsLocation.Resolve("https://host/game/StreamingAssets", "data.bytes") == "https://host/game/StreamingAssets/data.bytes", "WebGL URL");
            Check(StreamingAssetsLocation.Resolve("jar:file:///game/base.apk!/assets", "data.bytes") == "jar:file:///game/base.apk!/assets/data.bytes", "Android jar URL");
            foreach (var unsafePath in new[] { "", "../data.bytes", "nested/../data.bytes", "/data.bytes", "https://host/data", "nested//data", "./data" })
            {
                try { StreamingAssetsLocation.Resolve(root, unsafePath); throw new Exception("Unsafe path accepted: " + unsafePath); }
                catch (ArgumentException) { }
            }
            Check(StreamingAssetsLocation.CheckLocalFile(StreamingAssetsLocation.Resolve(root, "missing.bytes")).Code == "MASTERDATA-UNITY-MISSING", "Local missing phase");
            Directory.CreateDirectory(Path.Combine(root, "not-a-file"));
            Check(StreamingAssetsLocation.CheckLocalFile(StreamingAssetsLocation.Resolve(root, "not-a-file")).Code == "MASTERDATA-UNITY-READ", "Directory read phase");
            Check(StreamingAssetsLocation.CheckLocalFile("https://host/data.bytes") == null, "Network path used filesystem metadata");
            Console.WriteLine("PASS literal StreamingAssets paths, jar/HTTPS roots, unsafe paths and local missing/read phases");

            var selected = Path.Combine(root, "selected"); Directory.CreateDirectory(selected);
            Directory.CreateDirectory(Path.Combine(selected, "nested"));
            var source = Path.Combine(selected, "Item.cs");
            var nested = Path.Combine(selected, "nested", "Reward.cs");
            var metadata = Path.Combine(selected, "retired.cs.meta");
            var outside = Path.Combine(root, "Unrelated.cs");
            var binary = Path.Combine(root, "data.bytes");
            File.WriteAllText(source, "// first\r\n"); File.WriteAllText(nested, "// nested\n");
            File.WriteAllText(metadata, "user-owned metadata\n"); File.WriteAllText(outside, "not selected\n");
            File.WriteAllBytes(binary, bytes);
            var snapshot = ArtifactSnapshot.Read(selected, binary);
            Check(snapshot.CSharpFiles.SequenceEqual(new[] { source, nested }.OrderBy(path => path, StringComparer.Ordinal))
                && snapshot.MetadataFiles.SequenceEqual(new[] { metadata }) && snapshot.BinaryPresent && !snapshot.BinaryEmpty, "Scoped artifact presence");
            var before = Directory.GetFiles(root, "*", SearchOption.AllDirectories).ToDictionary(path => path, File.ReadAllBytes);
            var timestamp = File.GetLastWriteTimeUtc(source);
            File.WriteAllText(source, "// second\r\n"); File.SetLastWriteTimeUtc(source, timestamp);
            var changed = ArtifactSnapshot.Read(selected, binary);
            Check(snapshot.Signature != changed.Signature, "mtime became compile generation authority");
            Check(File.ReadAllBytes(metadata).SequenceEqual(before[metadata]) && File.ReadAllBytes(outside).SequenceEqual(before[outside])
                && File.ReadAllBytes(binary).SequenceEqual(bytes), "Observation changed unrelated bytes");
            File.WriteAllBytes(binary, Array.Empty<byte>());
            Check(ArtifactSnapshot.Read(selected, binary).BinaryEmpty, "Empty artifact not distinguished");
            File.Delete(binary); Check(!ArtifactSnapshot.Read(selected, binary).BinaryPresent, "Missing binary not distinguished");
            Console.WriteLine("PASS scoped source bytes, unchanged mtime freshness, metadata/unrelated preservation and binary states");

            var link = Path.Combine(selected, "linked-directory");
            var createdLink = false;
            try { Directory.CreateSymbolicLink(link, Path.Combine(root, "not-a-file")); createdLink = true; }
            catch (UnauthorizedAccessException) { Console.WriteLine("SKIP linked-scope creation: host lacks symlink permission"); }
            if (createdLink)
            {
                try { ArtifactSnapshot.Read(selected, binary); throw new Exception("Linked scope was followed"); }
                catch (IOException) { }
                finally { Directory.Delete(link); }
                Check(Directory.Exists(Path.Combine(root, "not-a-file")), "Linked target changed");
                Console.WriteLine("PASS linked C# scope fails closed without traversing or changing its target");
            }

            var firstAssembly = Path.Combine(root, "Library", "Selected.dll");
            var secondAssembly = Path.Combine(root, "Library", "Dependent.dll");
            var unrelatedAssembly = Path.Combine(root, "Library", "Unrelated.dll");
            var compiler = new CompileEvidence();
            Check(compiler.Current(snapshot.Signature, 1).state == "unknown", "Unobserved DLL treated as compile success");
            compiler.Begin(snapshot.Signature, 1, new[] { firstAssembly, secondAssembly });
            Check(compiler.Current(snapshot.Signature, 1).state == "pending", "Compile pending state");
            compiler.AssemblyFinished(unrelatedAssembly, new[] { "unrelated error" });
            compiler.AssemblyFinished(firstAssembly, Array.Empty<string>());
            Check(compiler.Complete(snapshot.Signature, 1).state == "unknown", "Partial callbacks treated as success");
            compiler.Begin(snapshot.Signature, 2, new[] { firstAssembly });
            compiler.AssemblyFinished(firstAssembly, Array.Empty<string>());
            Check(compiler.Complete(changed.Signature, 2).state == "unknown", "Stale generation won");
            compiler.Begin(changed.Signature, 3, new[] { firstAssembly });
            compiler.AssemblyFinished(firstAssembly, new[] { "exact selected compiler error" });
            var failure = compiler.Complete(changed.Signature, 3);
            Check(failure.state == "failed" && failure.messages.Single() == "exact selected compiler error", "Compile failure swallowed");
            compiler.Begin(changed.Signature, 4, new[] { firstAssembly, secondAssembly });
            compiler.AssemblyFinished(secondAssembly, Array.Empty<string>());
            compiler.AssemblyFinished(firstAssembly, Array.Empty<string>());
            var proof = compiler.Complete(changed.Signature, 4);
            var restored = new CompileEvidence(proof);
            Check(restored.Current(changed.Signature, 4).state == "observed"
                && restored.Current(snapshot.Signature, 4).state == "unknown"
                && restored.Current(changed.Signature, 5).state == "unknown", "Restored proof accepted changed source/compiler revision");
            compiler.Begin(changed.Signature, 5, Array.Empty<string>());
            Check(compiler.Complete(changed.Signature, 5).state == "unknown", "Unmapped source compile accepted");
            Console.WriteLine("PASS exact assembly callbacks, partial/stale rejection, selected errors and restored compile evidence");
        }
        finally { Directory.Delete(root, true); }
        Console.WriteLine("Portable boundary verification only; Unity Editor import/compile and Player execution are not observed.");
    }
}
