using System.Text.Json;
using Masterdata.MasterMemorySpike;
using MasterMemory;
using MessagePack;

[assembly: MasterMemoryGeneratorOptions(Namespace = "Masterdata.MasterMemorySpike")]

var outputPath = ParseOutputPath(args);
var records = new[]
{
    new SpikeItem { ItemId = 1001, Name = "Potion" },
    new SpikeItem { ItemId = 1002, Name = "Hi-Potion" },
    new SpikeItem { ItemId = 1003, Name = "Ether" },
};

var builder = new DatabaseBuilder();
builder.Append(records);
var binary = builder.Build();
Directory.CreateDirectory(Path.GetDirectoryName(outputPath) ?? ".");
File.WriteAllBytes(outputPath, binary);

var database = new MemoryDatabase(binary);
var reloaded = database.SpikeItemTable.FindByItemId(1002);
if (reloaded is null || reloaded.Name != "Hi-Potion")
{
    Console.Error.WriteLine("MasterMemory reload/lookup assertion failed.");
    return 1;
}

Console.WriteLine(JsonSerializer.Serialize(new
{
    status = "ok",
    masterMemoryVersion = "3.0.4",
    messagePackVersion = "3.1.3",
    binaryPath = Path.GetFullPath(outputPath),
    binarySize = binary.Length,
    reloadedItemId = reloaded.ItemId,
    reloadedItemName = reloaded.Name,
}));
return 0;

static string ParseOutputPath(string[] args)
{
    for (var index = 0; index + 1 < args.Length; index++)
    {
        if (args[index] == "--output")
        {
            return args[index + 1];
        }
    }

    return Path.Combine("target", "mastermemory-spike", "masterdata.bytes");
}

[MemoryTable("spike_item"), MessagePackObject(true)]
public record SpikeItem
{
    [PrimaryKey]
    public int ItemId { get; init; }

    public string Name { get; init; } = string.Empty;
}
