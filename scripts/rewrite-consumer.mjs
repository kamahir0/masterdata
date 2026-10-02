// Legacy CLI/.NET adapter. Expected public behavior lives in the portable corpus.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
const [binary, output] = process.argv.slice(2);
if (!binary || !output) throw Error('usage: node scripts/rewrite-consumer.mjs CLI_BINARY OUTPUT_JSON');
const corpus = path.resolve('fixtures/rewrite-oracle/v1/consumer');
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'masterdata-consumer-oracle-'));
const project = path.join(temp, 'input');
fs.cpSync('fixtures/full', project, {recursive:true});
for (const name of ['probe-schema.yaml','probe-data.yaml']) fs.copyFileSync(path.join(corpus,name),path.join(project,'sources',name));
const report = {method:'independent-public-CSharp-consumer-and-actual-MasterMemory-load',temp,platform:process.platform,stages:[]};
const run = (command,args) => {
  const result = spawnSync(command,args,{encoding:'utf8',maxBuffer:8*1024*1024});
  report.stages.push({command:path.basename(command),args,status:result.status,stdout:result.stdout,stderr:result.stderr});
  return result;
};
const hash = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
try {
  if (run(path.resolve(binary),['--project',project,'build']).status!==0) throw Error('canonical Build failed');
  const artifacts = path.join(project,'.masterdata/output');
  const before = hash(path.join(artifacts,'masterdata.bytes'));
  if (run(path.resolve(binary),['--project',project,'build']).status!==0) throw Error('repeat Build failed');
  report.deterministicBinary = before===hash(path.join(artifacts,'masterdata.bytes'));
  if (!report.deterministicBinary) throw Error('nondeterministic binary');
  const consumer = path.join(temp,'consumer');fs.mkdirSync(consumer);
  fs.cpSync(path.join(artifacts,'csharp'),path.join(consumer,'Generated'),{recursive:true});
  fs.copyFileSync(path.join(corpus,'Consumer.cs'),path.join(consumer,'Consumer.cs'));
  fs.writeFileSync(path.join(consumer,'consumer.csproj'),`<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><LangVersion>12</LangVersion><Nullable>enable</Nullable></PropertyGroup><ItemGroup><PackageReference Include="MasterMemory" Version="3.0.4"/><PackageReference Include="MessagePack" Version="3.1.3"/></ItemGroup></Project>`);
  fs.writeFileSync(path.join(consumer,'Program.cs'),'RewriteOracle.Consumer.Check(System.IO.File.ReadAllBytes(args[0]));\nSystem.Console.WriteLine("consumer oracle PASS");');
  const compiled = run('dotnet',['build',path.join(consumer,'consumer.csproj'),'-c','Release']);
  report.generatedCSharpCompiles = compiled.status===0;
  if (!report.generatedCSharpCompiles) throw Error('generated consumer compile failed');
  const loaded = run('dotnet',[path.join(consumer,'bin/Release/net8.0/consumer.dll'),path.join(artifacts,'masterdata.bytes')]);
  report.actualConsumerPass = loaded.status===0;
  if (!report.actualConsumerPass) throw Error('actual consumer oracle mismatch');
} catch (error) { report.error=String(error); process.exitCode=1; }
finally { fs.writeFileSync(output,JSON.stringify(report,null,2)); console.log(JSON.stringify({output,deterministicBinary:report.deterministicBinary,compiled:report.generatedCSharpCompiles,actualConsumerPass:report.actualConsumerPass,error:report.error})); }
