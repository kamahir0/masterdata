using System;
using UnityEditor;
using UnityEngine;
using UnityEngine.UIElements;

namespace MasterData.UnityEditorIntegration
{
    public sealed class DeliveryObservationWindow : EditorWindow
    {
        [SerializeField] private string csharpDirectory = "";
        [SerializeField] private string binaryFile = "";
        [SerializeField] private bool observing;
        private ArtifactObservation observation;
        private VisualElement status;
        [MenuItem("Window/MasterData/Delivery")]
        public static void Open() { GetWindow<DeliveryObservationWindow>("MasterData Delivery"); }
        public void CreateGUI()
        {
            rootVisualElement.Clear();
            var content = new ScrollView(); rootVisualElement.Add(content);
            var csharp = new TextField("Generated C# directory") { value = csharpDirectory, isDelayed = true, name = "csharp-directory" };
            var binary = new TextField("Binary file") { value = binaryFile, isDelayed = true, name = "binary-file" };
            csharp.AddToClassList(BaseField<string>.alignedFieldUssClassName);
            binary.AddToClassList(BaseField<string>.alignedFieldUssClassName);
            csharp.RegisterValueChangedCallback(change => { csharpDirectory = change.newValue; StopObservation(); Render(); });
            binary.RegisterValueChangedCallback(change => { binaryFile = change.newValue; StopObservation(); Render(); });
            content.Add(csharp); content.Add(binary);
            content.Add(new Button(StartObservation) { text = "Observe selected artifacts", name = "observe-artifacts" });
            status = new VisualElement { name = "delivery-status" }; content.Add(status);
            if (observing) StartObservation(); else Render();
        }
        private void StartObservation()
        {
            StopObservation();
            try
            {
                observation = new ArtifactObservation(csharpDirectory, binaryFile);
                observation.Changed += Render; observing = true; Render();
            }
            catch (Exception error)
            {
                status.Clear(); status.Add(new HelpBox("MASTERDATA-UNITY-SCOPE: " + error.Message, HelpBoxMessageType.Error));
            }
        }
        private void StopObservation()
        {
            observing = false;
            if (observation == null) return;
            observation.Changed -= Render; observation.Dispose(); observation = null;
        }
        private void OnDisable()
        {
            // Keep caller-selected paths and observation intent across domain
            // reload, while always releasing event subscriptions.
            var resume = observing; StopObservation(); observing = resume;
        }
        private void Render()
        {
            if (status == null) return;
            status.Clear();
            if (observation == null) { status.Add(new Label("Choose the exact generated C# directory and binary file.")); return; }
            var current = observation.Read();
            status.Add(new Label("Generated C#: " + (current.CSharpPresent ? "present" : "missing")));
            status.Add(new Label("Binary: " + (current.BinaryPresent ? "present" : "missing")));
            status.Add(new Label("Asset import: " + current.ImportState));
            status.Add(new Label("Compilation: " + current.CompileState));
            status.Add(new Label("Runtime load: not observed"));
            foreach (var diagnostic in current.Diagnostics)
                status.Add(new HelpBox(diagnostic.Code + ": " + diagnostic.Message,
                    diagnostic.Code.EndsWith("FAILED", StringComparison.Ordinal) || diagnostic.Code.EndsWith("READ", StringComparison.Ordinal)
                    || diagnostic.Code.EndsWith("EMPTY", StringComparison.Ordinal) ? HelpBoxMessageType.Error
                    : diagnostic.Code.EndsWith("PRESERVED", StringComparison.Ordinal) || diagnostic.Code.EndsWith("MISSING", StringComparison.Ordinal)
                    ? HelpBoxMessageType.Warning : HelpBoxMessageType.Info));
        }
    }
}
