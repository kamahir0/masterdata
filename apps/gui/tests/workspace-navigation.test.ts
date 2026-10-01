import { expect, test } from "vitest";
import { WorkspaceNavigator } from "../src/workspace-navigation";
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; }
test("rapid selection coalesces to the latest target before native work starts", async () => {
  const calls: string[] = [];
  const navigator = new WorkspaceNavigator<string>(async args => { calls.push(args.relativePath); return args.relativePath; });
  const requests = Array.from({ length: 50 }, (_, index) => navigator.select("/project", `source-${index}`));
  const results = await Promise.all(requests);
  expect(calls).toEqual(["source-49"]);
  expect(results.slice(0, -1).every(value => value === null)).toBe(true);
  expect(results.at(-1)).toBe("source-49");
});
test("one active read keeps only the newest pending request and discards obsolete results", async () => {
  const first = deferred<string>(); const calls: string[] = [];
  const navigator = new WorkspaceNavigator<string>(args => { calls.push(args.relativePath); return calls.length === 1 ? first.promise : Promise.resolve(args.relativePath); });
  const a = navigator.select("/project", "A"); await Promise.resolve();
  const b = navigator.select("/project", "B"); const c = navigator.select("/project", "C"); const d = navigator.select("/project", "D");
  expect(calls).toEqual(["A"]);
  first.resolve("A");
  expect(await Promise.all([a,b,c,d])).toEqual([null,null,null,"D"]);
  expect(calls).toEqual(["A","D"]);
});
test("Project change invalidates in-flight selection without reporting cancellation as failure", async () => {
  const first=deferred<string>(); const navigator=new WorkspaceNavigator<string>(()=>first.promise);
  const request=navigator.select("/old","A"); await Promise.resolve(); navigator.cancel(); first.resolve("old");
  expect(await request).toBeNull();
});
