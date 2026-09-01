using System.Collections;
using System.Collections.Immutable;
using System.Globalization;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;

if (args is ["--self-test"])
{
    var response = new
    {
        status = "ok",
        capability = "dotnet-bridge-smoke",
        masterMemoryBinaryBuild = "supported"
    };
    Console.WriteLine(JsonSerializer.Serialize(response));
    return 0;
}

string? reportPath = ArgumentParser.FindOption(args, "--report");
try
{
    var options = BuilderArguments.Parse(args);
    var report = Build(options);
    WriteReport(options.ReportPath, report);
    return 0;
}
catch (BuilderFailureException error)
{
    WriteErrorReport(reportPath, error.Phase, error.Message);
    Console.Error.WriteLine(error.Message);
    return 1;
}
catch (Exception error)
{
    WriteErrorReport(reportPath, "run", error.ToString());
    Console.Error.WriteLine(error);
    return 1;
}

static MasterMemoryBuildReport Build(BuilderArguments arguments)
{
    var request = ReadRequest(arguments.RequestPath);
    if (request.ProtocolVersion != 1)
    {
        throw Failure("prepare", $"unsupported builder protocol version {request.ProtocolVersion}");
    }
    if (!SamePath(request.OutputPath, arguments.OutputPath))
    {
        throw Failure(
            "prepare",
            $"request output `{request.OutputPath}` does not match command output `{arguments.OutputPath}`");
    }

    var assembly = Assembly.GetExecutingAssembly();
    var builderType = FindGeneratedType(assembly, request.Namespace, "DatabaseBuilder");
    var databaseType = FindGeneratedType(assembly, request.Namespace, "MemoryDatabase");
    var builder = Activator.CreateInstance(builderType)
        ?? throw Failure("run", "could not create the generated DatabaseBuilder");
    var tableReports = new List<MasterMemoryTableReport>(request.Tables.Count);

    foreach (var table in request.Tables)
    {
        var tableType = FindGeneratedType(assembly, request.Namespace, table.TypeName);
        var rows = Array.CreateInstance(tableType, table.Records.Count);
        for (var index = 0; index < table.Records.Count; index++)
        {
            rows.SetValue(CreateRow(tableType, table, table.Records[index]), index);
        }

        var append = builderType
            .GetMethods(BindingFlags.Public | BindingFlags.Instance)
            .Where(method => method.Name == "Append")
            .SingleOrDefault(method =>
            {
                var parameters = method.GetParameters();
                return parameters.Length == 1 && parameters[0].ParameterType.IsAssignableFrom(rows.GetType());
            });
        if (append is null)
        {
            throw Failure("run", $"generated DatabaseBuilder has no Append method for `{table.Identity}`");
        }

        Invoke(append, builder, rows);
    }

    var buildMethod = builderType.GetMethod("Build", BindingFlags.Public | BindingFlags.Instance)
        ?? throw Failure("run", "generated DatabaseBuilder has no Build method");
    var binary = Invoke(buildMethod, builder) as byte[]
        ?? throw Failure("run", "DatabaseBuilder.Build did not return binary data");
    if (binary.Length == 0)
    {
        throw Failure("run", "DatabaseBuilder returned an empty binary");
    }

    var database = CreateMemoryDatabase(databaseType, binary);
    ValidateDatabase(databaseType, database);
    foreach (var table in request.Tables)
    {
        var tableObject = GetGeneratedTable(databaseType, database, table.Identity)
            ?? throw Failure("run", $"generated MemoryDatabase has no table `{table.Identity}`");
        var actualCount = GetTableRecordCount(tableObject);
        if (actualCount != table.Records.Count)
        {
            throw Failure(
                "run",
                $"reloaded table `{table.Identity}` has {actualCount} record(s), expected {table.Records.Count}");
        }
        tableReports.Add(new MasterMemoryTableReport
        {
            Identity = table.Identity,
            RecordCount = actualCount
        });
    }

    var outputDirectory = Path.GetDirectoryName(arguments.OutputPath);
    if (!string.IsNullOrEmpty(outputDirectory))
    {
        Directory.CreateDirectory(outputDirectory);
    }
    File.WriteAllBytes(arguments.OutputPath, binary);

    return new MasterMemoryBuildReport
    {
        Status = "ok",
        ProtocolVersion = 1,
        MasterMemoryVersion = "3.0.4",
        MessagePackVersion = "3.1.3",
        BinaryPath = Path.GetFullPath(arguments.OutputPath),
        BinarySize = binary.LongLength,
        TableCount = request.Tables.Count,
        RecordCount = tableReports.Sum(table => table.RecordCount),
        Tables = tableReports
    };
}

static BuildRequest ReadRequest(string path)
{
    try
    {
        var request = JsonSerializer.Deserialize<BuildRequest>(File.ReadAllText(path), JsonOptions());
        return request ?? throw Failure("prepare", "builder request is empty");
    }
    catch (BuilderFailureException)
    {
        throw;
    }
    catch (Exception error)
    {
        throw Failure("prepare", $"could not read builder request: {error.Message}");
    }
}

static object CreateRow(Type tableType, BuildTable table, BuildRecord record)
{
    var row = Activator.CreateInstance(tableType)
        ?? throw Failure("run", $"could not create generated row `{table.TypeName}`");
    foreach (var field in table.Fields)
    {
        if (!record.Fields.TryGetValue(field.Name, out var value))
        {
            throw Failure("run", $"record for `{table.Identity}` is missing field `{field.Name}`");
        }
        var property = tableType.GetProperty(field.PropertyName, BindingFlags.Public | BindingFlags.Instance)
            ?? throw Failure(
                "run",
                $"generated row `{table.TypeName}` has no property `{field.PropertyName}`");
        var converted = ConvertValue(value, property.PropertyType);
        try
        {
            property.SetValue(row, converted);
        }
        catch (Exception error)
        {
            throw Failure(
                "run",
                $"could not set `{table.TypeName}.{field.PropertyName}`: {error.Message}");
        }
    }
    return row;
}

static object? ConvertValue(NormalizedValue value, Type targetType)
{
    if (value.Kind == "null")
    {
        if (targetType.IsValueType && Nullable.GetUnderlyingType(targetType) is null)
        {
            throw Failure("run", $"null cannot be assigned to `{targetType.FullName}`");
        }
        return null;
    }

    var nullableType = Nullable.GetUnderlyingType(targetType);
    if (nullableType is not null)
    {
        var inner = ConvertValue(value, nullableType);
        return Activator.CreateInstance(targetType, inner);
    }

    if (value.Kind == "array")
    {
        return ConvertArray(value, targetType);
    }

    return value.Kind switch
    {
        "bool" when targetType == typeof(bool) => value.Value.GetBoolean(),
        "string" when targetType == typeof(string) => value.Value.GetString()
            ?? throw Failure("run", "normalized string value is null"),
        "int" when targetType == typeof(int) => ParseInt32(value),
        "uint" when targetType == typeof(uint) => ParseUInt32(value),
        "long" when targetType == typeof(long) => ParseInt64(value),
        "ulong" when targetType == typeof(ulong) => ParseUInt64(value),
        "float" when targetType == typeof(float) => ParseSingle(value),
        "double" when targetType == typeof(double) => ParseDouble(value),
        "enum" or "flags" when targetType.IsEnum => ConvertEnum(value, targetType),
        "value_object" => ConvertValueObject(value, targetType),
        "custom" => ConvertCustom(value, targetType),
        _ => throw Failure(
            "run",
            $"normalized `{value.Kind}` value cannot be assigned to `{targetType.FullName}`")
    };
}

static object ConvertArray(NormalizedValue value, Type targetType)
{
    if (!targetType.IsGenericType
        || targetType.GetGenericTypeDefinition() != typeof(ImmutableArray<>))
    {
        throw Failure("run", $"normalized array cannot be assigned to `{targetType.FullName}`");
    }
    var elementType = targetType.GetGenericArguments()[0];
    var elements = value.Elements ?? throw Failure("run", "normalized array has no elements");
    var array = Array.CreateInstance(elementType, elements.Count);
    for (var index = 0; index < elements.Count; index++)
    {
        array.SetValue(ConvertValue(elements[index], elementType), index);
    }

    var createRange = typeof(ImmutableArray)
        .GetMethods(BindingFlags.Public | BindingFlags.Static)
        .Single(method =>
            method.Name == "CreateRange"
            && method.IsGenericMethodDefinition
            && method.GetParameters().Length == 1);
    return Invoke(createRange.MakeGenericMethod(elementType), null, array)
        ?? throw Failure("run", "could not create ImmutableArray value");
}

static object ConvertValueObject(NormalizedValue value, Type targetType)
{
    var nested = DeserializeNestedValue(value.Value);
    var constructor = targetType.GetConstructors()
        .SingleOrDefault(candidate => candidate.GetParameters().Length == 1)
        ?? throw Failure("run", $"generated Value Object `{targetType.Name}` has no one-argument constructor");
    var parameterType = constructor.GetParameters()[0].ParameterType;
    return InvokeConstructor(constructor, ConvertValue(nested, parameterType))
        ?? throw Failure("run", $"could not construct Value Object `{targetType.Name}`");
}

static object ConvertCustom(NormalizedValue value, Type targetType)
{
    var fields = value.Fields ?? throw Failure("run", "normalized Custom Type has no fields");
    var constructor = targetType.GetConstructors()
        .SingleOrDefault(candidate => candidate.GetParameters().Length == fields.Count)
        ?? throw Failure("run", $"generated Custom Type `{targetType.Name}` has no matching constructor");
    var parameters = constructor.GetParameters();
    var arguments = new object?[parameters.Length];
    for (var index = 0; index < parameters.Length; index++)
    {
        var parameter = parameters[index];
        if (parameter.Name is null || !fields.TryGetValue(parameter.Name, out var field))
        {
            throw Failure(
                "run",
                $"normalized Custom Type `{targetType.Name}` has no constructor field `{parameter.Name}");
        }
        arguments[index] = ConvertValue(field, parameter.ParameterType);
    }
    return InvokeConstructor(constructor, arguments)
        ?? throw Failure("run", $"could not construct Custom Type `{targetType.Name}`");
}

static object ConvertEnum(NormalizedValue value, Type targetType)
{
    var raw = value.Value.GetString()
        ?? throw Failure("run", "normalized Enum/Flags value is not a decimal string");
    var underlying = Enum.GetUnderlyingType(targetType);
    object parsed = underlying == typeof(int)
        ? ParseSigned32BitPattern(raw)
        : underlying == typeof(uint)
            ? uint.Parse(raw, NumberStyles.None, CultureInfo.InvariantCulture)
            : underlying == typeof(long)
                ? ParseSigned64BitPattern(raw)
                : underlying == typeof(ulong)
                    ? ulong.Parse(raw, NumberStyles.None, CultureInfo.InvariantCulture)
                    : throw Failure("run", $"unsupported generated Enum underlying type `{underlying}`");
    return Enum.ToObject(targetType, parsed);
}

static int ParseSigned32BitPattern(string raw)
{
    if (raw.StartsWith("-", StringComparison.Ordinal))
    {
        return int.Parse(raw, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture);
    }
    return unchecked((int)ulong.Parse(raw, NumberStyles.None, CultureInfo.InvariantCulture));
}

static long ParseSigned64BitPattern(string raw)
{
    if (raw.StartsWith("-", StringComparison.Ordinal))
    {
        return long.Parse(raw, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture);
    }
    return unchecked((long)ulong.Parse(raw, NumberStyles.None, CultureInfo.InvariantCulture));
}

static int ParseInt32(NormalizedValue value) => int.Parse(
    value.Value.GetString() ?? throw Failure("run", "normalized int value is not a string"),
    NumberStyles.AllowLeadingSign,
    CultureInfo.InvariantCulture);

static uint ParseUInt32(NormalizedValue value) => uint.Parse(
    value.Value.GetString() ?? throw Failure("run", "normalized uint value is not a string"),
    NumberStyles.None,
    CultureInfo.InvariantCulture);

static long ParseInt64(NormalizedValue value) => long.Parse(
    value.Value.GetString() ?? throw Failure("run", "normalized long value is not a string"),
    NumberStyles.AllowLeadingSign,
    CultureInfo.InvariantCulture);

static ulong ParseUInt64(NormalizedValue value) => ulong.Parse(
    value.Value.GetString() ?? throw Failure("run", "normalized ulong value is not a string"),
    NumberStyles.None,
    CultureInfo.InvariantCulture);

static float ParseSingle(NormalizedValue value) => float.Parse(
    value.Value.GetString() ?? throw Failure("run", "normalized float value is not a string"),
    NumberStyles.Float,
    CultureInfo.InvariantCulture);

static double ParseDouble(NormalizedValue value) => double.Parse(
    value.Value.GetString() ?? throw Failure("run", "normalized double value is not a string"),
    NumberStyles.Float,
    CultureInfo.InvariantCulture);

static object CreateMemoryDatabase(Type databaseType, byte[] binary)
{
    var constructor = databaseType.GetConstructors()
        .SingleOrDefault(candidate =>
        {
            var parameters = candidate.GetParameters();
            return parameters.Length > 0 && parameters[0].ParameterType == typeof(byte[]);
        })
        ?? throw Failure("run", "generated MemoryDatabase has no binary constructor");
    var parameters = constructor.GetParameters();
    var arguments = new object?[parameters.Length];
    arguments[0] = binary;
    for (var index = 1; index < parameters.Length; index++)
    {
        if (!parameters[index].HasDefaultValue)
        {
            throw Failure("run", $"MemoryDatabase constructor parameter `{parameters[index].Name}` has no default");
        }
        arguments[index] = parameters[index].DefaultValue;
    }
    return InvokeConstructor(constructor, arguments)
        ?? throw Failure("run", "could not reload generated MemoryDatabase");
}

static void ValidateDatabase(Type databaseType, object database)
{
    var validate = databaseType.GetMethod("Validate", BindingFlags.Public | BindingFlags.Instance, Type.EmptyTypes);
    if (validate is null)
    {
        return;
    }
    var result = Invoke(validate, database);
    if (result is null)
    {
        throw Failure("run", "MemoryDatabase.Validate returned no result");
    }
    var failedProperty = result.GetType().GetProperty("IsValidationFailed");
    if (failedProperty?.GetValue(result) is true)
    {
        throw Failure("run", "MemoryDatabase.Validate reported invalid binary");
    }
    if (result.GetType().GetProperty("FailedResults")?.GetValue(result) is IEnumerable failures
        && failures.Cast<object>().Any())
    {
        throw Failure("run", "MemoryDatabase.Validate reported failed table validation");
    }
}

static object? GetGeneratedTable(Type databaseType, object database, string identity)
{
    var getTable = databaseType.GetMethod("GetTable", BindingFlags.Public | BindingFlags.Static)
        ?? throw Failure("run", "generated MemoryDatabase has no table lookup method");
    return Invoke(getTable, null, database, identity);
}

static int GetTableRecordCount(object table)
{
    var rawData = table.GetType().GetMethod("GetRawDataUnsafe", BindingFlags.Public | BindingFlags.Instance)
        ?.Invoke(table, null);
    if (rawData is Array array)
    {
        return array.Length;
    }
    if (rawData is ICollection collection)
    {
        return collection.Count;
    }
    throw Failure("run", $"generated table `{table.GetType().Name}` did not expose raw records");
}

static Type FindGeneratedType(Assembly assembly, string namespaceName, string typeName)
{
    return assembly.GetType($"{namespaceName}.{typeName}", throwOnError: false, ignoreCase: false)
        ?? throw Failure("run", $"generated type `{namespaceName}.{typeName}` was not found");
}

static object? Invoke(MethodInfo method, object? instance, params object?[] arguments)
{
    try
    {
        return method.Invoke(instance, arguments);
    }
    catch (TargetInvocationException error) when (error.InnerException is not null)
    {
        throw Failure("run", error.InnerException.Message);
    }
    catch (Exception error)
    {
        throw Failure("run", error.Message);
    }
}

static object? InvokeConstructor(ConstructorInfo constructor, params object?[] arguments)
{
    try
    {
        return constructor.Invoke(arguments);
    }
    catch (TargetInvocationException error) when (error.InnerException is not null)
    {
        throw Failure("run", error.InnerException.Message);
    }
    catch (Exception error)
    {
        throw Failure("run", error.Message);
    }
}

static NormalizedValue DeserializeNestedValue(JsonElement value)
{
    try
    {
        return JsonSerializer.Deserialize<NormalizedValue>(value.GetRawText(), JsonOptions())
            ?? throw Failure("run", "normalized nested value is empty");
    }
    catch (BuilderFailureException)
    {
        throw;
    }
    catch (Exception error)
    {
        throw Failure("run", $"could not deserialize normalized nested value: {error.Message}");
    }
}

static BuilderFailureException Failure(string phase, string message) => new(phase, message);

static JsonSerializerOptions JsonOptions() => new()
{
    PropertyNameCaseInsensitive = false,
    PropertyNamingPolicy = JsonNamingPolicy.CamelCase
};

static bool SamePath(string left, string right)
{
    var leftFull = Path.GetFullPath(left);
    var rightFull = Path.GetFullPath(right);
    return OperatingSystem.IsWindows()
        ? string.Equals(leftFull, rightFull, StringComparison.OrdinalIgnoreCase)
        : string.Equals(leftFull, rightFull, StringComparison.Ordinal);
}

static void WriteReport(string path, MasterMemoryBuildReport report)
{
    File.WriteAllText(path, JsonSerializer.Serialize(report, JsonOptions()));
}

static void WriteErrorReport(string? path, string phase, string message)
{
    if (string.IsNullOrEmpty(path))
    {
        return;
    }
    try
    {
        File.WriteAllText(
            path,
            JsonSerializer.Serialize(
                new BuilderErrorReport
                {
                    Status = "error",
                    ProtocolVersion = 1,
                    Phase = phase,
                    Message = message
                },
                JsonOptions()));
    }
    catch
    {
        // The original builder failure remains the actionable diagnostic.
    }
}

sealed class BuilderArguments
{
    public required string RequestPath { get; init; }
    public required string OutputPath { get; init; }
    public required string ReportPath { get; init; }

    public static BuilderArguments Parse(string[] args)
    {
        var request = ArgumentParser.FindOption(args, "--request");
        var output = ArgumentParser.FindOption(args, "--output");
        var report = ArgumentParser.FindOption(args, "--report");
        if (string.IsNullOrEmpty(request) || string.IsNullOrEmpty(output) || string.IsNullOrEmpty(report))
        {
            throw new BuilderFailureException("prepare", "builder requires --request, --output, and --report");
        }
        return new BuilderArguments
        {
            RequestPath = request,
            OutputPath = output,
            ReportPath = report
        };
    }
}

sealed class BuilderFailureException(string phase, string message) : Exception(message)
{
    public string Phase { get; } = phase;
}

sealed class BuildRequest
{
    [JsonPropertyName("protocolVersion")]
    public int ProtocolVersion { get; set; }
    [JsonPropertyName("namespace")]
    public string Namespace { get; set; } = string.Empty;
    [JsonPropertyName("outputPath")]
    public string OutputPath { get; set; } = string.Empty;
    public List<BuildTable> Tables { get; set; } = [];
}

sealed class BuildTable
{
    public string Identity { get; set; } = string.Empty;
    [JsonPropertyName("typeName")]
    public string TypeName { get; set; } = string.Empty;
    public List<BuildField> Fields { get; set; } = [];
    public List<BuildRecord> Records { get; set; } = [];
}

sealed class BuildField
{
    public uint Key { get; set; }
    public string Name { get; set; } = string.Empty;
    [JsonPropertyName("propertyName")]
    public string PropertyName { get; set; } = string.Empty;
}

sealed class BuildRecord
{
    public Dictionary<string, NormalizedValue> Fields { get; set; } = [];
}

sealed class NormalizedValue
{
    public string Kind { get; set; } = string.Empty;
    public JsonElement Value { get; set; }
    [JsonPropertyName("type_name")]
    public string? TypeName { get; set; }
    public Dictionary<string, NormalizedValue>? Fields { get; set; }
    public List<NormalizedValue>? Elements { get; set; }
}

sealed class MasterMemoryBuildReport
{
    public string Status { get; set; } = string.Empty;
    [JsonPropertyName("protocolVersion")]
    public int ProtocolVersion { get; set; }
    [JsonPropertyName("masterMemoryVersion")]
    public string MasterMemoryVersion { get; set; } = string.Empty;
    [JsonPropertyName("messagePackVersion")]
    public string MessagePackVersion { get; set; } = string.Empty;
    [JsonPropertyName("binaryPath")]
    public string BinaryPath { get; set; } = string.Empty;
    [JsonPropertyName("binarySize")]
    public long BinarySize { get; set; }
    [JsonPropertyName("tableCount")]
    public int TableCount { get; set; }
    [JsonPropertyName("recordCount")]
    public int RecordCount { get; set; }
    public List<MasterMemoryTableReport> Tables { get; set; } = [];
}

sealed class MasterMemoryTableReport
{
    public string Identity { get; set; } = string.Empty;
    [JsonPropertyName("recordCount")]
    public int RecordCount { get; set; }
}

sealed class BuilderErrorReport
{
    public string Status { get; set; } = string.Empty;
    [JsonPropertyName("protocolVersion")]
    public int ProtocolVersion { get; set; }
    public string Phase { get; set; } = string.Empty;
    public string Message { get; set; } = string.Empty;
}

static class ArgumentParser
{
    public static string? FindOption(string[] args, string name)
    {
        for (var index = 0; index + 1 < args.Length; index++)
        {
            if (args[index] == name)
            {
                return args[index + 1];
            }
        }
        return null;
    }
}
