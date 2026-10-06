using System;
using System.IO;
using System.Linq;

namespace MasterData.UnityRuntime
{
    internal static class StreamingAssetsLocation
    {
        internal static string Resolve(string streamingAssetsRoot, string relativePath)
        {
            if (string.IsNullOrWhiteSpace(relativePath) || Path.IsPathRooted(relativePath)
                || Uri.TryCreate(relativePath, UriKind.Absolute, out _))
                throw new ArgumentException("Specify a StreamingAssets relative path.", nameof(relativePath));
            var parts = relativePath.Replace('\\', '/').Split('/');
            if (parts.Any(part => part == ".." || part == "." || part.Length == 0))
                throw new ArgumentException("StreamingAssets path contains an unsafe component.", nameof(relativePath));
            var root = streamingAssetsRoot.Replace('\\', '/').TrimEnd('/') + "/";
            if (!Uri.TryCreate(root, UriKind.Absolute, out var baseUrl)) baseUrl = new Uri(Path.GetFullPath(root));
            // Literal filenames are escaped once; Android jar and WebGL roots
            // remain URLs consumed by UnityWebRequest rather than File reads.
            return baseUrl.AbsoluteUri + string.Join("/", parts.Select(Uri.EscapeDataString));
        }
        internal static LoadError CheckLocalFile(string url)
        {
            var uri = new Uri(url);
            if (!uri.IsFile) return null;
            try
            {
                if ((File.GetAttributes(uri.LocalPath) & FileAttributes.Directory) != 0)
                    return new LoadError("MASTERDATA-UNITY-READ", "StreamingAssets target is a directory.");
                return null;
            }
            catch (FileNotFoundException error) { return Missing(error); }
            catch (DirectoryNotFoundException error) { return Missing(error); }
            catch (Exception error) { return new LoadError("MASTERDATA-UNITY-READ", "StreamingAssets target could not be inspected.", error); }
        }
        private static LoadError Missing(Exception error) { return new LoadError("MASTERDATA-UNITY-MISSING", "StreamingAssets target is missing.", error); }
    }
}
