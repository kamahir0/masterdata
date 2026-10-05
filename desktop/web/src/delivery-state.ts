import { desktop } from "./workspace";
import type { Diagnostic } from "./types";

export interface Captured {
  project: {id:string;name:string;version:string};
  root:string;
  configIdentity:string;
  inputIdentity:Record<string,string>;
}
export interface DeliveryError {code:string;message:string;diagnostics:Diagnostic[];diagnosticTotal:number}
export interface BuildResult {
  outcome:string; profile:string|null; dryRun:boolean; artifactRoot:string;
  rowCounts:Record<string,number>; receipt:unknown; nativeEvidence:string; message:string;
  configIdentity:string; retainedBackup:string|null;
}
export interface BuildAttempt {id:number;profile:string|null;captured:Captured|null;status:string;result:BuildResult|null;error:DeliveryError|null}
export interface TargetPreview {kind:string;configuredPath:string;destination:string;additions:string[];updates:string[];removals:string[];binaryReplacement:boolean}
export interface PublishPreview {artifactRoot:string;artifactIdentity:string;configIdentity:string;projectId:string;sourceFreshness:string;targets:TargetPreview[]}
export interface TargetResult {kind:string;destination:string;outcome:string;status:string;message:string;recoveryDirectory:string|null}
export interface PublishResult {outcome:string;sourceFreshness:string;unityVerification:string;noOp:boolean;targets:TargetResult[]}
export interface DeliveryContext {
  project:{id:string;name:string}; configIdentity:string; profiles:string[];
  targets:{kind:string;path:string}[];
  buildCapability:{available:boolean;reason:string|null};
  receipt:{eligible:boolean;root:string;identity:string|null;reason:string|null};
}
export interface Snapshot {
  epoch:number;id:number;kind:string|null;phase:string|null;status:string;
  running:boolean;mutating:boolean;capturing:boolean;profile:string|null;
  root:string|null;captured:Captured|null;context:DeliveryContext|null;
  result:BuildResult|PublishResult|Record<string,unknown>|null;error:DeliveryError|null;
  lastBuild:BuildAttempt|null;preview:{token:string;detail:PublishPreview}|null;
}
type Request = {operation:"context"|"publishPreview"|"cancelPreview"|"recheckArtifacts"}|
  {operation:"build";profile:string|null;dryRun:boolean}|{operation:"confirmPublish";token:string};
interface View {epoch:number;open:boolean;profile:string|null;pending:boolean;snapshot:Snapshot|null;error:string|null;combinedBuild:number|null}
const empty = (epoch:number):View=>({epoch,open:false,profile:null,pending:false,snapshot:null,error:null,combinedBuild:null});

// A delivery job has a separate publication channel: progress polling cannot
// publish a new grid projection or subscribe thousands of display cells.
class Delivery {
  view:View=empty(0);
  private listeners=new Set<()=>void>();
  private poll:ReturnType<typeof setTimeout>|null=null;
  private sequence=0;
  private origin:HTMLElement|null=null;
  subscribe=(f:()=>void)=>{this.listeners.add(f);return()=>{this.listeners.delete(f);};};
  snapshot=()=>this.view;
  private publish(delta:Partial<View>){this.view={...this.view,...delta};for(const f of this.listeners)f();}
  project(epoch:number){
    if(this.view.epoch===epoch)return;
    this.sequence++;if(this.poll)clearTimeout(this.poll);this.poll=null;
    this.view=empty(epoch);desktop.deliveryGate(false,false);for(const f of this.listeners)f();
  }
  open=()=>{
    this.project(desktop.surface.status.epoch);
    this.origin=document.activeElement as HTMLElement;
    this.publish({open:true});
    if(!this.view.snapshot?.running&&!this.view.pending)void this.request({operation:"context"});
  };
  close=()=>{this.publish({open:false});if(this.origin?.isConnected)this.origin.focus();};
  profile=(profile:string|null)=>this.publish({profile});
  private accept(snapshot:Snapshot){
    if(snapshot.epoch!==this.view.epoch||snapshot.epoch!==desktop.surface.status.epoch)return;
    desktop.deliveryGate(snapshot.mutating,snapshot.capturing);
    this.publish({snapshot,pending:false});
  }
  async request(request:Request):Promise<Snapshot|null>{
    if(this.view.pending||this.view.snapshot?.running)return null;
    const epoch=this.view.epoch,sequence=++this.sequence;
    if(this.poll)clearTimeout(this.poll);this.poll=null;
    this.publish({pending:true,error:null});
    if(request.operation==="build"||request.operation==="confirmPublish")
      desktop.deliveryGate(true,request.operation==="build");
    try {
      const reply=await desktop.rpc<Snapshot>({kind:"deliveryStart",epoch,request});
      if(epoch!==desktop.surface.status.epoch||sequence!==this.sequence)return null;
      this.accept(reply.data);
      if(reply.data.running)this.schedule(epoch,sequence);
      return reply.data;
    } catch(error) {
      if(epoch!==desktop.surface.status.epoch||sequence!==this.sequence)return null;
      this.publish({pending:false,error:describe(error)});
      // A lost IPC reply cannot authorize another mutation. Only read the job's
      // actual state; never repeat Build / Confirm Publish automatically.
      await this.observe(epoch,sequence);
      return null;
    }
  }
  private schedule(epoch:number,sequence:number){this.poll=setTimeout(()=>{this.poll=null;void this.observe(epoch,sequence);},150);}
  private async observe(epoch:number,sequence:number){
    try {
      const reply=await desktop.rpc<Snapshot>({kind:"deliveryState",epoch});
      if(sequence!==this.sequence||epoch!==desktop.surface.status.epoch)return;
      this.accept(reply.data);
      if(reply.data.running)this.schedule(epoch,sequence);
      else if(this.view.combinedBuild===reply.data.id&&reply.data.kind==="build"&&reply.data.status==="succeeded"&&
        (reply.data.result as BuildResult|null)?.outcome==="Success") {
        await this.request({operation:"publishPreview"});
      }
    } catch(error){if(sequence===this.sequence&&epoch===desktop.surface.status.epoch)this.publish({error:`結果を確認できません。自動再試行はしません。${describe(error)}`});}
  }
  refresh=()=>this.request({operation:"context"});
  recheck=()=>{const epoch=this.view.epoch,sequence=++this.sequence;if(this.poll)clearTimeout(this.poll);this.poll=null;return this.observe(epoch,sequence);};
  async build(combined=false){
    if(this.view.pending||this.view.snapshot?.running)return;
    this.publish({combinedBuild:null});
    const started=await this.request({operation:"build",profile:this.view.profile,dryRun:false});
    if(combined&&started&&started.epoch===this.view.epoch) {
      this.publish({combinedBuild:started.id});
      if(!started.running&&started.status==="succeeded")await this.request({operation:"publishPreview"});
    }
  }
  preview=()=>this.request({operation:"publishPreview"});
  cancel=()=>this.request({operation:"cancelPreview"});
  confirm=()=>{const preview=this.view.snapshot?.preview;return preview?this.request({operation:"confirmPublish",token:preview.token}):Promise.resolve(null);};
  artifacts=()=>this.request({operation:"recheckArtifacts"});
  async problems(start:number){
    const attempt=this.view.snapshot?.lastBuild;
    if(!attempt)return null;
    const epoch=this.view.epoch;
    const reply=await desktop.rpc<DeliveryError>({kind:"deliveryProblems",epoch,id:attempt.id,start,count:200});
    return epoch===this.view.epoch&&attempt.id===this.view.snapshot?.lastBuild?.id?reply.data:null;
  }
}
const describe=(error:unknown)=>error&&typeof error==="object"&&"message"in error?
  `${"code"in error?String(error.code)+": ":""}${String(error.message)}`:String(error);
export const delivery=new Delivery();
