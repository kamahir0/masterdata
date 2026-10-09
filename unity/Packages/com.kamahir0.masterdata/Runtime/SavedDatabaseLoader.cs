using System;
using System.Threading;

namespace MasterData.UnityRuntime
{
    public sealed class LoadError
    {
        public string Code { get; }
        public string Message { get; }
        public Exception Cause { get; }
        public LoadError(string code, string message, Exception cause = null)
        {
            Code = code;
            Message = message;
            Cause = cause;
        }
    }

    public sealed class LoadResult<T> where T : class
    {
        public T Database { get; }
        public LoadError Error { get; }
        public bool Succeeded => Error == null;
        private LoadResult(T database, LoadError error) { Database = database; Error = error; }
        public static LoadResult<T> Success(T database)
        {
            if (database == null) throw new ArgumentNullException(nameof(database));
            return new LoadResult<T>(database, null);
        }
        public static LoadResult<T> Failure(LoadError error)
        {
            if (error == null) throw new ArgumentNullException(nameof(error));
            return new LoadResult<T>(null, error);
        }
    }

    // UNITY-DELIVERY-005 / 006: only the consumer factory understands the binary.
    public static class SavedDatabaseLoader
    {
        public static LoadResult<T> LoadBytes<T>(byte[] bytes, Func<byte[], T> factory,
            CancellationToken cancellation = default) where T : class
        {
            if (cancellation.IsCancellationRequested) return Cancelled<T>();
            if (bytes == null) return LoadResult<T>.Failure(new LoadError(
                "MASTERDATA-UNITY-MISSING", "Database bytes are missing."));
            if (bytes.Length == 0) return LoadResult<T>.Failure(new LoadError(
                "MASTERDATA-UNITY-EMPTY", "Database bytes are empty."));
            if (factory == null) return LoadResult<T>.Failure(new LoadError(
                "MASTERDATA-UNITY-FACTORY", "A database factory is required."));
            try
            {
                var database = factory(bytes);
                if (cancellation.IsCancellationRequested) return Cancelled<T>();
                return database == null
                    ? LoadResult<T>.Failure(new LoadError("MASTERDATA-UNITY-FACTORY", "Database factory returned null."))
                    : LoadResult<T>.Success(database);
            }
            catch (OperationCanceledException error) { return Cancelled<T>(error); }
            catch (Exception error)
            {
                return LoadResult<T>.Failure(new LoadError("MASTERDATA-UNITY-FACTORY", "Database factory failed.", error));
            }
        }

        internal static LoadResult<T> Cancelled<T>(Exception cause = null) where T : class
        {
            return LoadResult<T>.Failure(new LoadError("MASTERDATA-UNITY-CANCELLED", "Database load was cancelled.", cause));
        }
    }
}
