using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Compilation;
using UnityEngine;

namespace MasterData.Unity.Editor
{
    public enum MasterDataUnityArtifactState
    {
        Missing,
        Present,
    }

    public enum MasterDataUnityImportState
    {
        Unknown,
        Pending,
        Observed,
    }

    public enum MasterDataUnityCompileState
    {
        Unknown,
        Compiling,
        Succeeded,
        Failed,
    }

    public enum MasterDataUnityDiagnosticSeverity
    {
        Info,
        Warning,
        Error,
    }

    public sealed class MasterDataUnityDiagnostic
    {
        public string Code { get; }
        public MasterDataUnityDiagnosticSeverity Severity { get; }
        public string Message { get; }
        public string Path { get; }

        public MasterDataUnityDiagnostic(
            string code,
            MasterDataUnityDiagnosticSeverity severity,
            string message,
            string path = null)
        {
            Code = code;
            Severity = severity;
            Message = message;
            Path = path;
        }
    }

    public sealed class MasterDataUnityDeliveryStatus
    {
        public string CSharpDirectory { get; }
        public string BinaryPath { get; }
        public MasterDataUnityArtifactState CSharpArtifact { get; }
        public MasterDataUnityArtifactState BinaryArtifact { get; }
        public MasterDataUnityImportState ImportState { get; }
        public MasterDataUnityCompileState CompileState { get; }
        public IReadOnlyList<MasterDataUnityDiagnostic> Diagnostics { get; }

        internal MasterDataUnityDeliveryStatus(
            string csharpDirectory,
            string binaryPath,
            MasterDataUnityArtifactState csharpArtifact,
            MasterDataUnityArtifactState binaryArtifact,
            MasterDataUnityImportState importState,
            MasterDataUnityCompileState compileState,
            IReadOnlyList<MasterDataUnityDiagnostic> diagnostics)
        {
            CSharpDirectory = csharpDirectory;
            BinaryPath = binaryPath;
            CSharpArtifact = csharpArtifact;
            BinaryArtifact = binaryArtifact;
            ImportState = importState;
            CompileState = compileState;
            Diagnostics = diagnostics;
        }
    }

    public static class MasterDataUnityDeliveryObserver
    {
        public static MasterDataUnityDeliveryStatus Observe(
            string csharpDirectory,
            string binaryPath)
        {
            if (string.IsNullOrWhiteSpace(csharpDirectory))
            {
                throw new ArgumentException("C# generated directory is required.", nameof(csharpDirectory));
            }

            if (string.IsNullOrWhiteSpace(binaryPath))
            {
                throw new ArgumentException("Binary path is required.", nameof(binaryPath));
            }

            var resolvedCSharpDirectory = Path.GetFullPath(csharpDirectory);
            var resolvedBinaryPath = Path.GetFullPath(binaryPath);
            var diagnostics = new List<MasterDataUnityDiagnostic>();
            var csharpFiles = Directory.Exists(resolvedCSharpDirectory)
                ? Directory.GetFiles(resolvedCSharpDirectory, "*.cs", SearchOption.AllDirectories)
                    .OrderBy(path => path, StringComparer.Ordinal)
                    .ToArray()
                : Array.Empty<string>();
            var hasCSharpArtifact = csharpFiles.Length > 0;
            var hasBinaryArtifact = File.Exists(resolvedBinaryPath);

            if (!hasCSharpArtifact)
            {
                diagnostics.Add(new MasterDataUnityDiagnostic(
                    "MASTERDATA-UNITY-ARTIFACT-CSHARP-MISSING",
                    MasterDataUnityDiagnosticSeverity.Error,
                    "No generated C# asset was found at the selected directory.",
                    resolvedCSharpDirectory));
            }

            if (!hasBinaryArtifact)
            {
                diagnostics.Add(new MasterDataUnityDiagnostic(
                    "MASTERDATA-UNITY-ARTIFACT-BINARY-MISSING",
                    MasterDataUnityDiagnosticSeverity.Error,
                    "The published MasterData binary was not found at the selected path.",
                    resolvedBinaryPath));
            }

            var csharpImported = hasCSharpArtifact && ObserveCSharpImport(csharpFiles, diagnostics);
            var binaryImported = hasBinaryArtifact && ObserveBinaryImport(resolvedBinaryPath, diagnostics);
            var importState = ResolveImportState(
                hasCSharpArtifact,
                hasBinaryArtifact,
                csharpImported,
                binaryImported,
                diagnostics);

            var compileState = MasterDataUnityCompilationObserver.State;
            if (EditorApplication.isCompiling)
            {
                compileState = MasterDataUnityCompileState.Compiling;
            }
            diagnostics.AddRange(MasterDataUnityCompilationObserver.Diagnostics);
            if (compileState == MasterDataUnityCompileState.Unknown)
            {
                diagnostics.Add(new MasterDataUnityDiagnostic(
                    "MASTERDATA-UNITY-COMPILE-UNKNOWN",
                    MasterDataUnityDiagnosticSeverity.Info,
                    "Unity compilation has not been observed by the MasterData integration."));
            }

            return new MasterDataUnityDeliveryStatus(
                resolvedCSharpDirectory,
                resolvedBinaryPath,
                hasCSharpArtifact ? MasterDataUnityArtifactState.Present : MasterDataUnityArtifactState.Missing,
                hasBinaryArtifact ? MasterDataUnityArtifactState.Present : MasterDataUnityArtifactState.Missing,
                importState,
                compileState,
                diagnostics);
        }

        private static bool ObserveCSharpImport(
            IReadOnlyList<string> csharpFiles,
            ICollection<MasterDataUnityDiagnostic> diagnostics)
        {
            var allObserved = true;
            foreach (var file in csharpFiles)
            {
                var assetPath = ToAssetPath(file);
                if (assetPath == null)
                {
                    allObserved = false;
                    diagnostics.Add(new MasterDataUnityDiagnostic(
                        "MASTERDATA-UNITY-IMPORT-UNKNOWN",
                        MasterDataUnityDiagnosticSeverity.Warning,
                        "The generated C# path is outside the Unity Assets tree; import was not observed.",
                        file));
                    continue;
                }

                if (AssetDatabase.LoadAssetAtPath<MonoScript>(assetPath) == null)
                {
                    allObserved = false;
                    diagnostics.Add(new MasterDataUnityDiagnostic(
                        "MASTERDATA-UNITY-IMPORT-PENDING",
                        MasterDataUnityDiagnosticSeverity.Warning,
                        "Unity has not imported the selected generated C# asset.",
                        assetPath));
                }
            }

            return allObserved;
        }

        private static bool ObserveBinaryImport(
            string binaryPath,
            ICollection<MasterDataUnityDiagnostic> diagnostics)
        {
            var assetPath = ToAssetPath(binaryPath);
            if (assetPath == null)
            {
                diagnostics.Add(new MasterDataUnityDiagnostic(
                    "MASTERDATA-UNITY-IMPORT-UNKNOWN",
                    MasterDataUnityDiagnosticSeverity.Warning,
                    "The binary path is outside the Unity Assets tree; import was not observed.",
                    binaryPath));
                return false;
            }

            if (AssetDatabase.LoadAssetAtPath<TextAsset>(assetPath) == null)
            {
                diagnostics.Add(new MasterDataUnityDiagnostic(
                    "MASTERDATA-UNITY-IMPORT-PENDING",
                    MasterDataUnityDiagnosticSeverity.Warning,
                    "Unity has not imported the selected binary asset.",
                    assetPath));
                return false;
            }

            return true;
        }

        private static MasterDataUnityImportState ResolveImportState(
            bool hasCSharpArtifact,
            bool hasBinaryArtifact,
            bool csharpImported,
            bool binaryImported,
            IEnumerable<MasterDataUnityDiagnostic> diagnostics)
        {
            if (!hasCSharpArtifact || !hasBinaryArtifact)
            {
                return MasterDataUnityImportState.Unknown;
            }

            if (diagnostics.Any(diagnostic => diagnostic.Code == "MASTERDATA-UNITY-IMPORT-UNKNOWN" ||
                                              diagnostic.Code == "MASTERDATA-UNITY-IMPORT-PENDING"))
            {
                return MasterDataUnityImportState.Pending;
            }

            return csharpImported && binaryImported
                ? MasterDataUnityImportState.Observed
                : MasterDataUnityImportState.Unknown;
        }

        private static string ToAssetPath(string absolutePath)
        {
            var assetsRoot = Path.GetFullPath(Application.dataPath)
                .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            var candidate = Path.GetFullPath(absolutePath);
            if (!candidate.StartsWith(assetsRoot + Path.DirectorySeparatorChar, StringComparison.Ordinal) &&
                !string.Equals(candidate, assetsRoot, StringComparison.Ordinal))
            {
                return null;
            }

            var relative = candidate.Substring(assetsRoot.Length).TrimStart(
                Path.DirectorySeparatorChar,
                Path.AltDirectorySeparatorChar);
            return string.IsNullOrEmpty(relative)
                ? "Assets"
                : "Assets/" + relative.Replace(Path.DirectorySeparatorChar, '/').Replace('\\', '/');
        }
    }

    [InitializeOnLoad]
    internal static class MasterDataUnityCompilationObserver
    {
        private static readonly List<MasterDataUnityDiagnostic> CurrentDiagnostics =
            new List<MasterDataUnityDiagnostic>();

        static MasterDataUnityCompilationObserver()
        {
            CompilationPipeline.compilationStarted += OnCompilationStarted;
            CompilationPipeline.compilationFinished += OnCompilationCompleted;
            CompilationPipeline.assemblyCompilationFinished += OnCompilationFinished;
        }

        public static MasterDataUnityCompileState State { get; private set; } = MasterDataUnityCompileState.Unknown;

        public static IReadOnlyList<MasterDataUnityDiagnostic> Diagnostics => CurrentDiagnostics;

        private static bool compilationHadError;

        private static void OnCompilationStarted(object context)
        {
            CurrentDiagnostics.Clear();
            compilationHadError = false;
            State = MasterDataUnityCompileState.Compiling;
        }

        private static void OnCompilationFinished(string assemblyPath, CompilerMessage[] messages)
        {
            var errors = messages == null
                ? Array.Empty<CompilerMessage>()
                : messages.Where(message => message.type == CompilerMessageType.Error).ToArray();
            if (errors.Length == 0)
            {
                return;
            }

            compilationHadError = true;
            foreach (var error in errors.OrderBy(message => message.file, StringComparer.Ordinal))
            {
                CurrentDiagnostics.Add(new MasterDataUnityDiagnostic(
                    "MASTERDATA-UNITY-COMPILE-FAILED",
                    MasterDataUnityDiagnosticSeverity.Error,
                    error.message,
                    error.file));
            }
        }

        private static void OnCompilationCompleted(object context)
        {
            State = compilationHadError
                ? MasterDataUnityCompileState.Failed
                : MasterDataUnityCompileState.Succeeded;
        }
    }
}
