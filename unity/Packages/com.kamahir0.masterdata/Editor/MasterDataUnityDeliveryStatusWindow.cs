using System.IO;
using UnityEditor;
using UnityEngine;

namespace MasterData.Unity.Editor
{
    public sealed class MasterDataUnityDeliveryStatusWindow : EditorWindow
    {
        private string csharpDirectory;
        private string binaryPath;
        private MasterDataUnityDeliveryStatus status;

        [MenuItem("MasterData/Delivery Status")]
        public static void Open()
        {
            GetWindow<MasterDataUnityDeliveryStatusWindow>("MasterData Delivery");
        }

        private void OnGUI()
        {
            EditorGUILayout.LabelField("MasterData Unity delivery", EditorStyles.boldLabel);
            EditorGUILayout.HelpBox(
                "Publish success is separate from Unity import, compile, and runtime load. Select the exact published assets to observe them.",
                MessageType.Info);

            csharpDirectory = EditorGUILayout.TextField("Generated C# directory", csharpDirectory ?? string.Empty);
            binaryPath = EditorGUILayout.TextField("Binary path", binaryPath ?? string.Empty);

            using (new EditorGUILayout.HorizontalScope())
            {
                if (GUILayout.Button("Select C# directory"))
                {
                    var selected = EditorUtility.OpenFolderPanel(
                        "Select generated C# directory",
                        Application.dataPath,
                        string.Empty);
                    if (!string.IsNullOrEmpty(selected))
                    {
                        csharpDirectory = selected;
                    }
                }

                if (GUILayout.Button("Select binary"))
                {
                    var selected = EditorUtility.OpenFilePanel(
                        "Select MasterData binary",
                        Application.dataPath,
                        "bytes");
                    if (!string.IsNullOrEmpty(selected))
                    {
                        binaryPath = selected;
                    }
                }
            }

            if (GUILayout.Button("Observe selected delivery"))
            {
                Observe();
            }

            if (status == null)
            {
                return;
            }

            EditorGUILayout.Space(8);
            EditorGUILayout.LabelField("C# artifact", status.CSharpArtifact.ToString());
            EditorGUILayout.LabelField("Binary artifact", status.BinaryArtifact.ToString());
            EditorGUILayout.LabelField("Import", status.ImportState.ToString());
            EditorGUILayout.LabelField("Compile", status.CompileState.ToString());
            EditorGUILayout.LabelField("Selected C#", status.CSharpDirectory);
            EditorGUILayout.LabelField("Selected binary", status.BinaryPath);

            if (status.Diagnostics.Count == 0)
            {
                EditorGUILayout.HelpBox("No Unity diagnostics were observed.", MessageType.Info);
                return;
            }

            EditorGUILayout.Space(8);
            EditorGUILayout.LabelField("Diagnostics", EditorStyles.boldLabel);
            foreach (var diagnostic in status.Diagnostics)
            {
                var messageType = diagnostic.Severity == MasterDataUnityDiagnosticSeverity.Error
                    ? MessageType.Error
                    : diagnostic.Severity == MasterDataUnityDiagnosticSeverity.Warning
                        ? MessageType.Warning
                        : MessageType.Info;
                var path = string.IsNullOrEmpty(diagnostic.Path) ? string.Empty : " [" + diagnostic.Path + "]";
                EditorGUILayout.HelpBox(diagnostic.Code + ": " + diagnostic.Message + path, messageType);
            }
        }

        private void Observe()
        {
            if (string.IsNullOrWhiteSpace(csharpDirectory) || string.IsNullOrWhiteSpace(binaryPath))
            {
                status = null;
                return;
            }

            status = MasterDataUnityDeliveryObserver.Observe(
                Path.GetFullPath(csharpDirectory),
                Path.GetFullPath(binaryPath));
        }
    }
}
