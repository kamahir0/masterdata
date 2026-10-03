using System;
using System.Collections.Generic;
using MasterMemory;
using Masterdata.Generated;
[assembly: MasterMemoryGeneratorOptions(Namespace = "Masterdata.Generated")]
namespace RewriteOracle
{
    public static class Consumer
    {
        public static void Check(byte[] bytes)
        {
            var database = new MemoryDatabase(bytes);
            var failures = new List<string>();
            Action<string, Action> verify = (name, check) => {
                try { check(); Console.WriteLine("PASS " + name); }
                catch (Exception error) { failures.Add(name + ": " + error.Message); }
            };
            verify("required reference helper", () => {
                if (database.ProbeTable.FindById(1).GetParent(database).Id != 1001) throw new Exception("expected item 1001");
            });
            var first = database.ItemMasterTable.FindById(1001);
            var second = database.ItemMasterTable.FindById(1002);
            verify("PK lookup", () => {
                if (first.Name != "Sword" || second.Name != "Debug Sword") throw new Exception("wrong names");
            });
            verify("64-bit boundaries", () => {
                if (first.LongValue != long.MinValue || second.LongValue != long.MaxValue || first.UlongValue != ulong.MaxValue) throw new Exception("rounded value");
            });
            verify("nested Value Object", () => {
                if (first.Reward.ItemId.Value != 2001 || second.Reward.ItemId.Value != 2002) throw new Exception("expected 2001, actual " + first.Reward.ItemId.Value);
            });
            verify("direct Value Object", () => {
                if (first.ItemId.Value != 2001 || second.ItemId.Value != 2002) throw new Exception("wrong direct values");
            });
            verify("Custom multi-field preservation", () => {
                if (first.Reward.Values.Length != 1 || first.Reward.Values[0] != 1 || second.Reward.Values.Length != 2 || second.Reward.Values[0] != 2 || second.Reward.Values[1] != 3 || first.Reward.Amount != 1 || second.Reward.Amount != uint.MaxValue || first.Reward.Note != null || second.Reward.Note != null) throw new Exception("Custom values corrupted");
            });
            verify("Array values", () => {
                if (first.Numbers.Length != 2 || first.Numbers[0] != 1 || first.Numbers[1] != -2) throw new Exception("wrong elements");
            });
            verify("secondary key", () => {
                if (database.ItemMasterTable.FindByRarity(Rarity.Rare).Id != 1002) throw new Exception("expected item 1002");
            });
            verify("nonunique Value Object key", () => {
                var count = 0;
                foreach (var row in database.ItemMasterTable.FindByCode(new ItemCode("sword"))) count++;
                if (count != 2) throw new Exception("expected 2 matches, actual " + count);
            });
            verify("composite key", () => {
                if (database.ItemMasterTable.FindByCodeAndRarity((new ItemCode("sword"), Rarity.Common)).Id != 1001) throw new Exception("expected item 1001");
            });
            if (failures.Count != 0) throw new Exception(string.Join("; ", failures));
        }
    }
}
