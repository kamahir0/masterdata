using System;
using System.Collections;
using System.IO;
using System.Threading;
using MasterData.UnityRuntime;
using MasterData.UnityEditorIntegration;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace MasterData.UnityTests
{
    public sealed class DeliveryTests
    {
        private sealed class Database { public readonly byte[] Bytes; public Database(byte[] bytes) { Bytes = bytes; } }
        [Test]
        public void ByteFactoryIsCallerOwnedAndFailuresRemainStructured()
        {
            var bytes = new byte[] { 1, 255 };
            var a = SavedDatabaseLoader.LoadBytes(bytes, data => new Database(data));
            var b = SavedDatabaseLoader.LoadBytes(bytes, data => new Database(data));
            Assert.That(a.Succeeded && b.Succeeded);
            Assert.That(a.Database, Is.Not.SameAs(b.Database)); Assert.That(a.Database.Bytes, Is.SameAs(bytes));
            Assert.That(SavedDatabaseLoader.LoadBytes<Database>(null, data => new Database(data)).Error.Code, Is.EqualTo("MASTERDATA-UNITY-MISSING"));
            Assert.That(SavedDatabaseLoader.LoadBytes(Array.Empty<byte>(), data => new Database(data)).Error.Code, Is.EqualTo("MASTERDATA-UNITY-EMPTY"));
            Assert.That(SavedDatabaseLoader.LoadBytes<Database>(bytes, _ => throw new InvalidOperationException()).Error.Code, Is.EqualTo("MASTERDATA-UNITY-FACTORY"));
            using (var cancellation = new CancellationTokenSource())
            {
                cancellation.Cancel();
                Assert.That(SavedDatabaseLoader.LoadBytes(bytes, data => new Database(data), cancellation.Token).Error.Code, Is.EqualTo("MASTERDATA-UNITY-CANCELLED"));
            }
        }
        [UnityTest]
        public IEnumerator StreamingAssetsReadsExplicitBytesAndReportsMissingAndEmpty()
        {
            var relative = "masterdata-test-" + Guid.NewGuid().ToString("N") + "/data.bytes";
            var file = Path.Combine(Application.streamingAssetsPath, relative);
            Directory.CreateDirectory(Path.GetDirectoryName(file)); File.WriteAllBytes(file, new byte[] { 7, 9 });
            try
            {
                LoadResult<Database> result = null;
                yield return StreamingAssetsDatabase.Load(relative, data => new Database(data), value => result = value);
                Assert.That(result.Succeeded); Assert.That(result.Database.Bytes, Is.EqualTo(new byte[] { 7, 9 }));
                File.WriteAllBytes(file, Array.Empty<byte>());
                yield return StreamingAssetsDatabase.Load(relative, data => new Database(data), value => result = value);
                Assert.That(result.Error.Code, Is.EqualTo("MASTERDATA-UNITY-EMPTY"));
                File.Delete(file);
                yield return StreamingAssetsDatabase.Load(relative, data => new Database(data), value => result = value);
                Assert.That(result.Error.Code, Is.EqualTo("MASTERDATA-UNITY-MISSING"));
            }
            finally { Directory.Delete(Path.GetDirectoryName(file), true); }
        }
        [UnityTest]
        public IEnumerator AttachedWindowKeepsExplicitInputAndStatusInsideNarrowAndWideBounds()
        {
            var window = ScriptableObject.CreateInstance<DeliveryObservationWindow>();
            window.Show();
            try
            {
                foreach (var width in new[] { 360f, 960f })
                {
                    window.position = new Rect(80, 80, width, 420); yield return null; yield return null;
                    foreach (var field in window.rootVisualElement.Query<TextField>().ToList())
                    {
                        Assert.That(field.worldBound.width, Is.GreaterThan(0));
                        Assert.That(field.worldBound.xMin, Is.GreaterThanOrEqualTo(window.rootVisualElement.worldBound.xMin - 1));
                        Assert.That(field.worldBound.xMax, Is.LessThanOrEqualTo(window.rootVisualElement.worldBound.xMax + 1));
                    }
                    Assert.That(window.rootVisualElement.Q<Button>("observe-artifacts"), Is.Not.Null);
                    Assert.That(window.rootVisualElement.Q<VisualElement>("delivery-status").worldBound.yMin,
                        Is.GreaterThanOrEqualTo(window.rootVisualElement.Q<TextField>("binary-file").worldBound.yMax));
                }
            }
            finally { window.Close(); }
        }
    }
}
