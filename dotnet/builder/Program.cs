using System.Text.Json;

if (args is ["--self-test"])
{
    var response = new
    {
        status = "ok",
        capability = "dotnet-bridge-smoke",
        masterMemoryBinaryBuild = "not-implemented"
    };
    Console.WriteLine(JsonSerializer.Serialize(response));
    return 0;
}

Console.Error.WriteLine("The builder currently supports only --self-test.");
Console.Error.WriteLine("MasterMemory v3 Source Generator integration is not implemented yet.");
return 2;

