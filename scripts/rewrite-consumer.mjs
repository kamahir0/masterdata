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
fs.appendFileSync(path.join(project,'masterdata.toml'),'\n[[publish.targets]]\nkind = "csharp"\npath = "dist/csharp"\n\n[[publish.targets]]\nkind = "binary"\npath = "dist/masterdata.bytes"\n');
for (const name of ['probe-schema.yaml','probe-data.yaml']) fs.copyFileSync(path.join(corpus,name),path.join(project,'sources',name));
const revision=spawnSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).stdout.trim();
const dirty=spawnSync('git',['status','--porcelain'],{encoding:'utf8'}).stdout.length!==0;
const report = {method:'independent-public-CSharp-consumer-and-actual-MasterMemory-load',executedAt:new Date().toISOString(),revision,workingTreeDirty:dirty,temp,platform:process.platform,stages:[]};
const run = (command,args) => {
  const result = spawnSync(command,args,{encoding:'utf8',maxBuffer:8*1024*1024});
  report.stages.push({command:path.basename(command),args,status:result.status,stdout:result.stdout,stderr:result.stderr});
  return result;
};
const hash = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const generatedHash = directory => {
  const digest=crypto.createHash('sha256');
  for(const name of fs.readdirSync(directory).sort()) { digest.update(name);digest.update(fs.readFileSync(path.join(directory,name))); }
  return digest.digest('hex');
};
try {
  if (run(path.resolve(binary),['--project',project,'build']).status!==0) throw Error('canonical Build failed');
  report.buildDoesNotPublish = !fs.existsSync(path.join(project,'dist'));
  if (!report.buildDoesNotPublish) throw Error('Build implicitly published');
  const artifacts = path.join(project,'.masterdata/output');
  const before = hash(path.join(artifacts,'masterdata.bytes'));
  const generatedBefore=generatedHash(path.join(artifacts,'csharp'));
  if (run(path.resolve(binary),['--project',project,'build']).status!==0) throw Error('repeat Build failed');
  report.deterministicBinary = before===hash(path.join(artifacts,'masterdata.bytes'));
  report.deterministicGeneratedCSharp=generatedBefore===generatedHash(path.join(artifacts,'csharp'));
  if (!report.deterministicBinary) throw Error('nondeterministic binary');
  if (!report.deterministicGeneratedCSharp) throw Error('nondeterministic generated C#');
  const dataFiles=fs.readdirSync(path.join(project,'sources')).filter(name=>name.endsWith('.yaml') && /^kind: data\r?\n/.test(fs.readFileSync(path.join(project,'sources',name),'utf8')));
  const changedSource = path.join(project,'sources',dataFiles[0]);
  const savedBytes=fs.readFileSync(changedSource);
  fs.writeFileSync(changedSource,'kind: data\nrecords: [broken\n');
  fs.mkdirSync(path.join(project,'dist/csharp'),{recursive:true});
  fs.writeFileSync(path.join(project,'dist/csharp/unmanaged.txt'),'caller-owned');
  fs.writeFileSync(path.join(project,'dist/masterdata.bytes.meta'),'unity-owned-guid');
  if(run(path.resolve(binary),['--project',project,'publish']).status!==0) throw Error('eligible artifact publish depended on current YAML');
  report.standalonePublishEligibleWithInvalidCurrentSource = hash(path.join(project,'dist/masterdata.bytes'))===before;
  report.unmanagedAndMetadataPreserved = fs.readFileSync(path.join(project,'dist/csharp/unmanaged.txt'),'utf8')==='caller-owned' && fs.readFileSync(path.join(project,'dist/masterdata.bytes.meta'),'utf8')==='unity-owned-guid';
  const canonicalBinary=fs.readFileSync(path.join(artifacts,'masterdata.bytes'));
  fs.writeFileSync(path.join(artifacts,'masterdata.bytes'),'corrupt');
  report.corruptReceiptArtifactRejected=run(path.resolve(binary),['--project',project,'publish']).status!==0 && hash(path.join(project,'dist/masterdata.bytes'))===before;
  fs.writeFileSync(path.join(artifacts,'masterdata.bytes'),canonicalBinary);
  fs.writeFileSync(changedSource,savedBytes);
  report.buildPublishComposition = run(path.resolve(binary),['--project',project,'build','--publish']).status===0 && hash(path.join(project,'dist/masterdata.bytes'))===before;
  for(const key of ['standalonePublishEligibleWithInvalidCurrentSource','unmanagedAndMetadataPreserved','corruptReceiptArtifactRejected','buildPublishComposition']) if(!report[key]) throw Error(`delivery oracle failed: ${key}`);
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
