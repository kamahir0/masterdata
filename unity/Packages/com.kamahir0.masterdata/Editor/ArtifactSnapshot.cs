using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

namespace MasterData.UnityEditorIntegration
{
    internal sealed class ArtifactSnapshot
    {
        public string[] CSharpFiles { get; }
        public string[] MetadataFiles { get; }
        public bool BinaryPresent { get; }
        public bool BinaryEmpty { get; }
        public string Signature { get; }
        private ArtifactSnapshot(string[] csharp, string[] metadata, bool binary, bool empty, string signature)
        { CSharpFiles = csharp; MetadataFiles = metadata; BinaryPresent = binary; BinaryEmpty = empty; Signature = signature; }

        public static ArtifactSnapshot Read(string exactCSharpDirectory, string exactBinaryFile)
        {
            var directory = Path.GetFullPath(exactCSharpDirectory);
            var files = new List<string>();
            var metadata = new List<string>();
            var pending = new Stack<string>();
            if (Directory.Exists(directory)) pending.Push(directory);
            while (pending.Count != 0)
            {
                var current = pending.Pop();
                if ((File.GetAttributes(current) & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("Linked C# directories are not observed.");
                foreach (var entry in Directory.EnumerateFileSystemEntries(current).OrderBy(path => path, StringComparer.Ordinal))
                {
                    var attributes = File.GetAttributes(entry);
                    if ((attributes & FileAttributes.ReparsePoint) != 0) throw new IOException("Linked artifacts are not observed.");
                    if ((attributes & FileAttributes.Directory) != 0) pending.Push(entry);
                    else if (Path.GetExtension(entry).Equals(".cs", StringComparison.OrdinalIgnoreCase)) files.Add(entry);
                    else if (Path.GetExtension(entry).Equals(".meta", StringComparison.OrdinalIgnoreCase)) metadata.Add(entry);
                }
            }
            files.Sort(StringComparer.Ordinal);
            metadata.Sort(StringComparer.Ordinal);
            using (var hash = SHA256.Create())
            {
                var signature = new StringBuilder(directory).Append('\n');
                foreach (var file in files)
                    signature.Append(file).Append(':').Append(Convert.ToBase64String(hash.ComputeHash(File.ReadAllBytes(file)))).Append('\n');
                var binary = new FileInfo(exactBinaryFile);
                return new ArtifactSnapshot(files.ToArray(), metadata.ToArray(), binary.Exists, binary.Exists && binary.Length == 0,
                    Convert.ToBase64String(hash.ComputeHash(Encoding.UTF8.GetBytes(signature.ToString()))));
            }
        }
    }

    [Serializable]
    internal sealed class CompileRecord
    {
        public string signature;
        public int revision;
        public string state = "unknown";
        public string[] messages = Array.Empty<string>();
    }

    // An existing DLL or a quiet compiler is not proof of the selected source
    // generation. Only captured, complete Unity callbacks advance this record.
    internal sealed class CompileEvidence
    {
        private readonly HashSet<string> expected = new HashSet<string>(PathComparer);
        private readonly HashSet<string> completed = new HashSet<string>(PathComparer);
        private readonly List<string> errors = new List<string>();
        private CompileRecord record;
        private bool active;
        public CompileEvidence(CompileRecord previous = null) { record = previous; }
        public void Begin(string signature, int revision, IEnumerable<string> exactAssemblies)
        {
            expected.Clear(); completed.Clear(); errors.Clear();
            foreach (var path in exactAssemblies) expected.Add(Path.GetFullPath(path));
            record = new CompileRecord { signature = signature, revision = revision, state = "pending" };
            active = true;
        }
        public void AssemblyFinished(string assembly, IEnumerable<string> failures)
        {
            if (!active || !expected.Contains(Path.GetFullPath(assembly))) return;
            completed.Add(Path.GetFullPath(assembly));
            errors.AddRange(failures);
        }
        public CompileRecord Complete(string currentSignature, int revision)
        {
            if (!active) return record;
            active = false;
            record.state = record.signature != currentSignature || record.revision != revision || expected.Count == 0
                ? "unknown" : errors.Count != 0 ? "failed" : expected.SetEquals(completed) ? "observed" : "unknown";
            record.messages = errors.ToArray();
            return record;
        }
        public CompileRecord Current(string currentSignature, int revision)
        {
            return record != null && record.signature == currentSignature && record.revision == revision
                ? record : new CompileRecord();
        }
        private static StringComparer PathComparer => Path.DirectorySeparatorChar == '\\' ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal;
    }
}
