using System;
using System.Collections;
using System.Threading;
using UnityEngine;
using UnityEngine.Networking;

namespace MasterData.UnityRuntime
{
    public static class StreamingAssetsDatabase
    {
        public static IEnumerator Load<T>(string relativePath, Func<byte[], T> factory,
            Action<LoadResult<T>> completed, CancellationToken cancellation = default) where T : class
        {
            if (completed == null) throw new ArgumentNullException(nameof(completed));
            if (cancellation.IsCancellationRequested) { completed(SavedDatabaseLoader.Cancelled<T>()); yield break; }
            UnityWebRequest request = null;
            LoadError failure = null;
            string url = null;
            try
            {
                url = StreamingAssetsLocation.Resolve(Application.streamingAssetsPath, relativePath);
                failure = StreamingAssetsLocation.CheckLocalFile(url);
                if (failure == null) request = UnityWebRequest.Get(url);
            }
            catch (Exception error) { failure = new LoadError("MASTERDATA-UNITY-READ", "StreamingAssets path could not be opened.", error); }
            if (failure != null) { completed(LoadResult<T>.Failure(failure)); yield break; }
            using (request)
            {
                UnityWebRequestAsyncOperation operation = null;
                try { operation = request.SendWebRequest(); }
                catch (Exception error) { failure = new LoadError("MASTERDATA-UNITY-READ", "StreamingAssets read could not start.", error); }
                if (failure != null) { completed(LoadResult<T>.Failure(failure)); yield break; }
                while (!operation.isDone)
                {
                    if (cancellation.IsCancellationRequested)
                    {
                        request.Abort();
                        completed(SavedDatabaseLoader.Cancelled<T>());
                        yield break;
                    }
                    yield return null;
                }
                if (cancellation.IsCancellationRequested) { completed(SavedDatabaseLoader.Cancelled<T>()); yield break; }
                if (request.result != UnityWebRequest.Result.Success)
                {
                    completed(LoadResult<T>.Failure(StreamingAssetsLocation.CheckLocalFile(url) ?? new LoadError(request.responseCode == 404
                        ? "MASTERDATA-UNITY-MISSING" : "MASTERDATA-UNITY-READ", request.error ?? "StreamingAssets read failed.")));
                    yield break;
                }
                completed(SavedDatabaseLoader.LoadBytes(request.downloadHandler.data, factory, cancellation));
            }
        }

    }
}
