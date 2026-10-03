// Legacy measurement host, not a rewrite API. LaunchServices supplies a visible
// foreground Tauri process; a separate read-only monitor checks the display session.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
const [bundle,fixture,output,mode='clean'] = process.argv.slice(2);
if(process.platform!=='darwin' || process.arch!=='arm64' || !output) throw Error('usage (macOS arm64): node scripts/rewrite-desktop-macos.mjs BUNDLE FIXTURE OUTPUT_DIRECTORY [clean|dirty-rapid]');
fs.mkdirSync(output,{recursive:true});
const absoluteOutput=path.resolve(output);
const candidate=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
const digest = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const sourceHash = root => {
  const h=crypto.createHash('sha256');
  const walk = (directory,prefix='') => {
    for(const name of fs.readdirSync(directory).sort()) {
      const file=path.join(directory,name), relative=prefix+name;
      if(fs.statSync(file).isDirectory()) walk(file,relative+'/');
      else {h.update(relative);h.update(fs.readFileSync(file));}
    }
  };walk(root);return h.digest('hex');
};
const monitorSource=path.join(absoluteOutput,'monitor.swift');
fs.writeFileSync(monitorSource,`import AppKit
import CoreGraphics
import Foundation
let expected = "dev.masterdata.navigation-evidence"
while true {
 let app = NSWorkspace.shared.frontmostApplication
 let pid = app?.processIdentifier ?? 0
 let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
 let visible = windows.contains { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }
 let value: [String: Any] = ["at": Date().timeIntervalSince1970 * 1000, "expectedForeground": app?.bundleIdentifier == expected, "visibleWindow": visible]
 let data = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
 FileHandle.standardOutput.write(data); FileHandle.standardOutput.write(Data([10]))
 RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.25))
}
`);
const monitorBinary=path.join(absoluteOutput,'monitor');
const compile=spawnSync('swiftc',[monitorSource,'-o',monitorBinary],{encoding:'utf8'});
if(compile.status!==0) throw Error(compile.stderr);
const stdout=fs.openSync(path.join(absoluteOutput,'foreground.jsonl'),'w');
const monitor=spawn(monitorBinary,[],{stdio:['ignore',stdout,'inherit']});
const awake=spawn('/usr/bin/caffeinate',['-dims'],{stdio:'ignore'});
const before=sourceHash(fixture);
const startedAt=new Date().toISOString();
const executable=path.join(bundle,'Contents/MacOS/masterdata-gui');
const host={method:'LaunchServices-foreground-with-native-WebView-and-OS-display-monitor',candidate,workingTreeDirty:execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).length!==0,
  platform:os.platform(),arch:os.arch(),osRelease:os.release(),cpu:os.cpus()[0].model,memoryBytes:os.totalmem(),node:process.version,
  bundleSha256:digest(executable),fixtureSha256:before,mode,startedAt,
  sleepInhibited:'caffeinate -dims',monitorPeriodMs:250,input:'synthetic DOM click/key in actual Tauri WebView',paint:'rAF opportunity, not GPU completion'};
try {
  const child=spawn('/usr/bin/open',['-n','-W',
    '--env',`MASTERDATA_PROJECT_PATH=${path.resolve(fixture)}`,
    '--env',`MASTERDATA_NAVIGATION_EVIDENCE_OUTPUT=${path.join(absoluteOutput,'desktop.json')}`,
    '--env',`MASTERDATA_NAVIGATION_EVIDENCE_MODE=${mode}`,
    '--env',`MASTERDATA_NAVIGATION_EVIDENCE_CANDIDATE=${candidate}`,
    '--stderr',path.join(absoluteOutput,'runtime.log'),path.resolve(bundle)],{stdio:'inherit'});
  const status=await new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',resolve);});
  host.launcherStatus=status;
  const evidence=JSON.parse(fs.readFileSync(path.join(absoluteOutput,'desktop.json'),'utf8'));
  host.completed=evidence.distributions?.length>0 && !evidence.error;
  for (const rapid of evidence.rapidRuns ?? []) {
    const last = rapid.trace.findLastIndex(event=>event.phase==='selection');
    if (rapid.active !== rapid.trace[last]?.path || rapid.trace.slice(last+1).some(event=>event.phase==='react-commit' && event.path!==rapid.active))
      throw Error('obsolete selection result became current');
  }
  const display = fs.readFileSync(path.join(absoluteOutput,'foreground.jsonl'),'utf8').trim().split('\n').filter(Boolean).map(JSON.parse);
  const first = display.findIndex(event=>event.expectedForeground && event.visibleWindow);
  const last = display.findLastIndex(event=>event.expectedForeground && event.visibleWindow);
  const controlled = display.slice(first,last+1);
  host.displayMonitor = {samples:controlled.length,invalid:controlled.filter(event=>!event.expectedForeground || !event.visibleWindow).length,excludedStartupQuitSamples:display.length-controlled.length};
  if (!controlled.length || host.displayMonitor.invalid) throw Error('measurement unavailable: OS foreground/visibility interruption');
  host.sourceBytesUnchanged=before===sourceHash(fixture);
  if(!host.sourceBytesUnchanged || !host.completed) throw Error(evidence.error ?? 'incomplete measurement');
} finally {
  host.endedAt=new Date().toISOString();
  monitor.kill('SIGTERM');awake.kill('SIGTERM');fs.closeSync(stdout);
  fs.writeFileSync(path.join(absoluteOutput,'host.json'),JSON.stringify(host,null,2));
}
console.log(JSON.stringify({output:absoluteOutput,completed:host.completed,candidate}));
