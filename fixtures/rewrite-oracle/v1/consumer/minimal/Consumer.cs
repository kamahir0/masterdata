using System;
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
            var first = database.MiniatureTable.FindById(1);
            var second = database.MiniatureTable.FindById(2);
            if (first.Direct.Value != 3001 || second.Direct.Value != 3002)
                throw new Exception("direct Value Object corrupted");
            if (first.Reward.ItemId.Value != 2001 || second.Reward.ItemId.Value != 2002)
                throw new Exception("nested Value Object corrupted");
            if (first.Reward.Amount != 17 || second.Reward.Amount != 19)
                throw new Exception("reordered Custom field corrupted");
            Console.WriteLine("PASS direct / nested / distinct records / declaration-key order");
        }
    }
}
