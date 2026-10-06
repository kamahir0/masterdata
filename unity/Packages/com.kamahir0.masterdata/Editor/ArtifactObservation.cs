using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using UnityEditor;
using UnityEditor.Compilation;
using UnityEngine;

namespace MasterData.UnityEditorIntegration
{
    public sealed class DeliveryDiagnostic
    {
        public string Code { get; }
        public string Message { get; }
        public DeliveryDiagnostic(string code, string message) { Code = code; Message = message; }
    }
    public sealed class DeliveryStatus
    {
        public bool CSharpPresent { get; internal set; }
        public bool BinaryPresent { get; internal set; }
        public string ImportState { get; internal set; } = "unknown";
        public string CompileState { get; internal set; } = "unknown";
        public DeliveryDiagnostic[] Diagnostics { get; internal set; } = Array.Empty<DeliveryDiagnostic>();
    }

    [InitializeOnLoad]
    internal static class CompilerRevision
    {
        private const string Key = "MasterData.Unity.CompileRevision";
        public static int Current => SessionState.GetInt(Key, 0);
        static CompilerRevision() { CompilationPipeline.compilationStarted += _ => SessionState.SetInt(Key, Current + 1); }
    }

    public sealed class ArtifactObservation : IDisposable
    {
        private readonly string projectRoot, csharpDirectory, binaryFile, sessionKey;
        private readonly CompileEvidence compiler;
        private string compilationSignature;
        public event Action Changed;
        public ArtifactObservation(string exactCSharpDirectory, string exactBinaryFile)
        {
            projectRoot = Directory.GetParent(Application.dataPath).FullName;
            csharpDirectory = ResolveAssetPath(exactCSharpDirectory);
            binaryFile = ResolveAssetPath(exactBinaryFile);
            using (var hash = SHA256.Create()) sessionKey = "MasterData.Unity.Observed." +
                Convert.ToBase64String(hash.ComputeHash(Encoding.UTF8.GetBytes(csharpDirectory + "\n" + binaryFile)));
            // Force revision subscription before per-scope compiler callbacks.
            _ = CompilerRevision.Current;
            var previous = SessionState.GetString(sessionKey, "");
            compiler = new CompileEvidence(string.IsNullOrEmpty(previous) ? null : JsonUtility.FromJson<CompileRecord>(previous));
            CompilationPipeline.compilationStarted += CompilationStarted;
            CompilationPipeline.assemblyCompilationFinished += AssemblyFinished;
            CompilationPipeline.compilationFinished += CompilationFinished;
            EditorApplication.projectChanged += ProjectChanged;
        }
        public void Dispose()
        {
            CompilationPipeline.compilationStarted -= CompilationStarted;
            CompilationPipeline.assemblyCompilationFinished -= AssemblyFinished;
            CompilationPipeline.compilationFinished -= CompilationFinished;
            EditorApplication.projectChanged -= ProjectChanged;
        }
        public DeliveryStatus Read()
        {
            var status = new DeliveryStatus();
            var diagnostics = new List<DeliveryDiagnostic>();
            try
            {
                var snapshot = ArtifactSnapshot.Read(csharpDirectory, binaryFile);
                status.CSharpPresent = snapshot.CSharpFiles.Length != 0;
                status.BinaryPresent = snapshot.BinaryPresent;
                if (!status.CSharpPresent) diagnostics.Add(new DeliveryDiagnostic("MASTERDATA-UNITY-CSHARP-MISSING", "Generated C# is missing from the selected directory."));
                if (!status.BinaryPresent) diagnostics.Add(new DeliveryDiagnostic("MASTERDATA-UNITY-BINARY-MISSING", "The selected binary is missing."));
                if (snapshot.BinaryEmpty) diagnostics.Add(new DeliveryDiagnostic("MASTERDATA-UNITY-BINARY-EMPTY", "The selected binary is empty."));
                status.ImportState = EditorApplication.isUpdating ? "pending"
                    : status.CSharpPresent && status.BinaryPresent && snapshot.CSharpFiles.All(file =>
                        AssetDatabase.LoadAssetAtPath<MonoScript>(AssetPath(file)) != null)
                        && AssetDatabase.LoadAssetAtPath<TextAsset>(AssetPath(binaryFile)) != null ? "observed" : "unknown";
                if (status.ImportState != "observed") diagnostics.Add(new DeliveryDiagnostic(status.ImportState == "pending"
                    ? "MASTERDATA-UNITY-IMPORT-PENDING" : "MASTERDATA-UNITY-IMPORT-UNKNOWN", "Unity asset import has not been fully observed."));
                foreach (var metadata in snapshot.MetadataFiles)
                {
                    var asset = metadata.Substring(0, metadata.Length - ".meta".Length);
                    if ((!File.Exists(asset) && !Directory.Exists(asset)) || string.IsNullOrEmpty(AssetDatabase.AssetPathToGUID(AssetPath(asset))))
                        diagnostics.Add(new DeliveryDiagnostic("MASTERDATA-UNITY-METADATA-PRESERVED", "Unverified Unity metadata is preserved: " + AssetPath(metadata)));
                }
                var record = compiler.Current(snapshot.Signature, CompilerRevision.Current);
                status.CompileState = EditorApplication.isCompiling ? "pending" : record.state;
                if (status.CompileState != "observed") diagnostics.Add(new DeliveryDiagnostic(status.CompileState == "failed"
                    ? "MASTERDATA-UNITY-COMPILE-FAILED" : status.CompileState == "pending"
                    ? "MASTERDATA-UNITY-COMPILE-PENDING" : "MASTERDATA-UNITY-COMPILE-UNKNOWN", "Selected C# compilation: " + status.CompileState));
                foreach (var message in record.messages ?? Array.Empty<string>()) diagnostics.Add(new DeliveryDiagnostic("MASTERDATA-UNITY-COMPILER-MESSAGE", message));
            }
            catch (Exception error) { diagnostics.Add(new DeliveryDiagnostic("MASTERDATA-UNITY-OBSERVATION-READ", error.Message)); }
            status.Diagnostics = diagnostics.ToArray();
            return status;
        }
        private void CompilationStarted(object context)
        {
            try
            {
                var snapshot = ArtifactSnapshot.Read(csharpDirectory, binaryFile);
                var files = new HashSet<string>(snapshot.CSharpFiles.Select(Path.GetFullPath), PathComparer);
                var covered = new HashSet<string>(PathComparer);
                var assemblies = new List<string>();
                foreach (var assembly in CompilationPipeline.GetAssemblies(AssembliesType.Editor))
                {
                    var matches = assembly.sourceFiles.Select(FullPath).Where(files.Contains).ToArray();
                    if (matches.Length == 0) continue;
                    covered.UnionWith(matches); assemblies.Add(FullPath(assembly.outputPath));
                }
                compilationSignature = snapshot.Signature;
                compiler.Begin(snapshot.Signature, CompilerRevision.Current, covered.SetEquals(files) ? assemblies : Array.Empty<string>());
            }
            catch { compilationSignature = null; compiler.Begin("", CompilerRevision.Current, Array.Empty<string>()); }
            Changed?.Invoke();
        }
        private void AssemblyFinished(string output, CompilerMessage[] messages)
        {
            compiler.AssemblyFinished(FullPath(output), messages.Where(message => message.type == CompilerMessageType.Error)
                .Select(message => message.file + ":" + message.line + " " + message.message));
        }
        private void CompilationFinished(object context)
        {
            string signature;
            try { signature = ArtifactSnapshot.Read(csharpDirectory, binaryFile).Signature; }
            catch { signature = null; }
            var record = compiler.Complete(compilationSignature == null ? null : signature, CompilerRevision.Current);
            if (record != null) SessionState.SetString(sessionKey, JsonUtility.ToJson(record));
            Changed?.Invoke();
        }
        private void ProjectChanged() { Changed?.Invoke(); }
        private string ResolveAssetPath(string path)
        {
            if (string.IsNullOrWhiteSpace(path)) throw new ArgumentException("Specify an exact Unity asset path.");
            var full = Path.GetFullPath(Path.IsPathRooted(path) ? path : Path.Combine(projectRoot, path));
            var relative = AssetPath(full);
            if (!relative.StartsWith("Assets/", StringComparison.Ordinal) && !relative.StartsWith("Packages/", StringComparison.Ordinal))
                throw new ArgumentException("The selected artifact must be inside this Unity project's Assets or embedded Packages.");
            return full;
        }
        private string AssetPath(string full) { return Path.GetRelativePath(projectRoot, full).Replace('\\', '/'); }
        private string FullPath(string path) { return Path.GetFullPath(Path.IsPathRooted(path) ? path : Path.Combine(projectRoot, path)); }
        private static StringComparer PathComparer => Path.DirectorySeparatorChar == '\\' ? StringComparer.OrdinalIgnoreCase : StringComparer.Ordinal;
    }
}
