using System;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;
using UnityEngine.Networking;

namespace MasterData.Unity
{
    /// <summary>
    /// Stable failure categories emitted by the Unity runtime adapter.
    /// MasterMemory construction failures are deliberately kept under the
    /// factory category so the package does not duplicate that dependency's
    /// exception model.
    /// </summary>
    public enum MasterDataRuntimeErrorCode
    {
        BinaryMissing,
        BinaryPathInvalid,
        BinaryReadFailed,
        BinaryEmpty,
        DatabaseFactoryFailed,
        OperationCanceled,
    }

    public sealed class MasterDataRuntimeException : Exception
    {
        public MasterDataRuntimeErrorCode Code { get; }
        public string DiagnosticCode { get; }

        public MasterDataRuntimeException(
            MasterDataRuntimeErrorCode code,
            string message,
            Exception innerException = null)
            : base(message, innerException)
        {
            Code = code;
            DiagnosticCode = DiagnosticCodeFor(code);
        }

        private static string DiagnosticCodeFor(MasterDataRuntimeErrorCode code)
        {
            switch (code)
            {
                case MasterDataRuntimeErrorCode.BinaryMissing:
                    return "MASTERDATA-UNITY-RUNTIME-BINARY-MISSING";
                case MasterDataRuntimeErrorCode.BinaryPathInvalid:
                    return "MASTERDATA-UNITY-RUNTIME-BINARY-PATH";
                case MasterDataRuntimeErrorCode.BinaryReadFailed:
                    return "MASTERDATA-UNITY-RUNTIME-BINARY-READ";
                case MasterDataRuntimeErrorCode.BinaryEmpty:
                    return "MASTERDATA-UNITY-RUNTIME-BINARY-EMPTY";
                case MasterDataRuntimeErrorCode.DatabaseFactoryFailed:
                    return "MASTERDATA-UNITY-RUNTIME-DATABASE-FACTORY";
                case MasterDataRuntimeErrorCode.OperationCanceled:
                    return "MASTERDATA-UNITY-RUNTIME-CANCELED";
                default:
                    return "MASTERDATA-UNITY-RUNTIME-UNKNOWN";
            }
        }
    }

    /// <summary>
    /// Thin runtime boundary for a receipt-published MasterData binary.
    /// The caller supplies the generated database factory and owns the
    /// resulting database instance and any subsequent reload.
    /// </summary>
    public static class MasterDataRuntime
    {
        public static TDatabase LoadDatabase<TDatabase>(
            byte[] binary,
            Func<byte[], TDatabase> databaseFactory)
        {
            if (binary == null || binary.Length == 0)
            {
                throw new MasterDataRuntimeException(
                    MasterDataRuntimeErrorCode.BinaryEmpty,
                    "MasterData binary is null or empty.");
            }

            if (databaseFactory == null)
            {
                throw new MasterDataRuntimeException(
                    MasterDataRuntimeErrorCode.DatabaseFactoryFailed,
                    "A generated database factory is required.");
            }

            // The generated MemoryDatabase constructor is the authority for
            // binary semantics. A private copy gives the factory a stable
            // input even when the caller reuses its download buffer.
            var ownedBinary = new byte[binary.Length];
            Buffer.BlockCopy(binary, 0, ownedBinary, 0, binary.Length);
            try
            {
                var database = databaseFactory(ownedBinary);
                if ((object)database == null)
                {
                    throw new MasterDataRuntimeException(
                        MasterDataRuntimeErrorCode.DatabaseFactoryFailed,
                        "The generated database factory returned null.");
                }

                return database;
            }
            catch (MasterDataRuntimeException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new MasterDataRuntimeException(
                    MasterDataRuntimeErrorCode.DatabaseFactoryFailed,
                    "The generated MasterMemory database factory rejected the MasterData binary.",
                    exception);
            }
        }

        public static async Task<TDatabase> LoadStreamingAssetsAsync<TDatabase>(
            string relativePath,
            Func<byte[], TDatabase> databaseFactory,
            CancellationToken cancellationToken = default(CancellationToken))
        {
            var normalizedPath = NormalizeRelativePath(relativePath);
            ThrowIfCanceled(cancellationToken);
            var uri = Application.streamingAssetsPath.TrimEnd('/', '\\') + "/" + normalizedPath;

            using (var request = UnityWebRequest.Get(uri))
            {
                var operation = request.SendWebRequest();
                while (!operation.isDone)
                {
                    if (cancellationToken.IsCancellationRequested)
                    {
                        request.Abort();
                        throw new MasterDataRuntimeException(
                            MasterDataRuntimeErrorCode.OperationCanceled,
                            "MasterData binary loading was canceled.");
                    }

                    await Task.Yield();
                }

                ThrowIfCanceled(cancellationToken);
                if (request.result != UnityWebRequest.Result.Success)
                {
                    var code = request.responseCode == 404
                        ? MasterDataRuntimeErrorCode.BinaryMissing
                        : MasterDataRuntimeErrorCode.BinaryReadFailed;
                    throw new MasterDataRuntimeException(
                        code,
                        "Could not read the MasterData binary from StreamingAssets: " +
                        (request.error ?? "unknown UnityWebRequest failure") + ".");
                }

                var data = request.downloadHandler == null ? null : request.downloadHandler.data;
                if (data == null || data.Length == 0)
                {
                    throw new MasterDataRuntimeException(
                        MasterDataRuntimeErrorCode.BinaryEmpty,
                        "The MasterData binary read from StreamingAssets is empty.");
                }

                return LoadDatabase(data, databaseFactory);
            }
        }

        public static string NormalizeRelativePath(string relativePath)
        {
            if (string.IsNullOrWhiteSpace(relativePath))
            {
                throw new MasterDataRuntimeException(
                    MasterDataRuntimeErrorCode.BinaryPathInvalid,
                    "The MasterData StreamingAssets path must not be empty.");
            }

            var normalized = relativePath.Replace('\\', '/');
            if (normalized.StartsWith("/", StringComparison.Ordinal) ||
                (normalized.Length >= 2 && normalized[1] == ':'))
            {
                throw new MasterDataRuntimeException(
                    MasterDataRuntimeErrorCode.BinaryPathInvalid,
                    "The MasterData StreamingAssets path must be relative.");
            }

            foreach (var segment in normalized.Split('/'))
            {
                if (segment == "..")
                {
                    throw new MasterDataRuntimeException(
                        MasterDataRuntimeErrorCode.BinaryPathInvalid,
                        "The MasterData StreamingAssets path must not escape its root.");
                }
            }

            return normalized;
        }

        private static void ThrowIfCanceled(CancellationToken cancellationToken)
        {
            if (cancellationToken.IsCancellationRequested)
            {
                throw new MasterDataRuntimeException(
                    MasterDataRuntimeErrorCode.OperationCanceled,
                    "MasterData binary loading was canceled.");
            }
        }
    }
}
