import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import TypeEditor from '../src/TypeEditor';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
const base={path:'type.yaml',name:'Rarity',category:'Enum',underlying:'ulong',conversions:null,members:[{name:'Rare',value:'18446744073709551615'},{name:'Common',value:'0'}],fields:[],fieldTypes:['int','ulong','string']};
let snapshot:any;let plan:any;let outcome:any;
const onResult=vi.fn(async()=>{});const beginApply=vi.fn(()=>true);const endApply=vi.fn();
beforeEach(()=>{snapshot=structuredClone(base);plan={token:'1',target:'Rarity',operation:'RenameEnumMember',selector:'Rare',destructive:false,affectedOccurrenceCount:2,files:[{path:'data.yaml',before:'Rare # before',after:'Epic # before'}],diagnostics:[]};outcome={state:'success',files:['data.yaml']};invoke.mockReset();onResult.mockClear();beginApply.mockClear();endApply.mockClear();invoke.mockImplementation(async command=>{
  if(command==='open_type')return snapshot;if(command==='plan_type_migration')return plan;if(command==='apply_table_migration')return outcome;throw Error(command);
});});
afterEach(cleanup);
async function open(dirtyPaths:string[]=[]) {render(<TypeEditor projectPath="/project" path="type.yaml" canWrite dirtyPaths={dirtyPaths} beginApply={beginApply} onResult={onResult} endApply={endApply}/>);await screen.findByRole('heading',{name:new RegExp(snapshot.name)});}
async function rename(){fireEvent.click(screen.getByRole('button',{name:'Rename Member',exact:true}));fireEvent.change(screen.getByLabelText('Member name'),{target:{value:'Epic'}});fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Type Migration Plan'});}
test('resolved enum displays exact numeric values and invalidates Plan on input changes',async()=>{await open();expect(screen.getByText('18446744073709551615')).toBeTruthy();await rename();expect(screen.getByText('Rare # before')).toBeTruthy();expect(screen.getByText('Epic # before')).toBeTruthy();fireEvent.change(screen.getByLabelText('Member name'),{target:{value:'Other'}});expect(screen.queryByRole('button',{name:'Apply reviewed Plan'})).toBeNull();});
// GUI-TYPE-STATE-003: 各ケースを独立したmountにして、2回分のAnt Design
// renderとApplyを1つの5秒budgetへ詰め込まない。実UIを使うためCIの描画差に
// 余裕を持たせるが、各waitForの期限と操作結果のassertionは維持する。
test('affected dirty file blocks Apply without starting a mutation', async () => {
  await open(['data.yaml']);
  await rename();
  expect(screen.getByText('Apply blocked by unsaved affected files')).toBeTruthy();
  const apply = screen.getByRole('button', { name: 'Apply reviewed Plan' });
  expect((apply as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(apply);
  expect(beginApply).not.toHaveBeenCalled();
  expect(invoke.mock.calls.some(([command]) => command === 'apply_table_migration')).toBe(false);
}, 10_000);

test('unrelated dirty file allows the reviewed Plan to finish applying', async () => {
  await open(['other.yaml']);
  await rename();
  const apply = screen.getByRole('button', { name: 'Apply reviewed Plan' });
  expect((apply as HTMLButtonElement).disabled).toBe(false);
  fireEvent.click(apply);
  // 再読込とfinallyまで待ち、次のtestへ未完了のApplyを持ち越さない。
  await waitFor(() => expect(endApply).toHaveBeenCalledOnce());
  expect(onResult).toHaveBeenCalledExactlyOnceWith(outcome);
  expect(beginApply).toHaveBeenCalledExactlyOnceWith(['data.yaml']);
  expect(invoke.mock.calls.find(([command]) => command === 'apply_table_migration')?.[1])
    .toEqual({ projectPath: '/project', token: '1', allowDestructive: false });
}, 10_000);
test('enum Add sends explicit numeric text without lossy coercion',async()=>{await open();fireEvent.click(screen.getByRole('button',{name:'Add Member',exact:true}));fireEvent.change(screen.getByLabelText('Member name'),{target:{value:'High'}});fireEvent.change(screen.getByLabelText('Numeric value'),{target:{value:'18446744073709551614'}});fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Type Migration Plan'});expect(invoke.mock.calls.find(([c])=>c==='plan_type_migration')?.[1].input).toEqual({operation:'add_enum',target:'Rarity',name:'High',value:'18446744073709551614'});});
test('Flags None cannot be renamed or dropped',async()=>{snapshot.category='Flags Enum';snapshot.members=[{name:'None',value:'0'},{name:'Fire',value:'1'}];await open();for(const action of ['Rename Member','Drop Member'])expect((screen.getByRole('button',{name:action,exact:true}) as HTMLButtonElement).disabled).toBe(true);fireEvent.click(screen.getByRole('radio',{name:'Select member Fire'}));expect((screen.getByRole('button',{name:'Rename Member',exact:true}) as HTMLButtonElement).disabled).toBe(false);});
test('Value Object conversions are independent semantic settings',async()=>{snapshot.category='Value Object';snapshot.conversions={fromUnderlyingImplicit:false,toUnderlyingImplicit:true};snapshot.members=[];await open();fireEvent.click(screen.getByRole('button',{name:'Edit conversions'}));fireEvent.click(screen.getByRole('checkbox',{name:'fromUnderlyingImplicit',exact:true}));fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Type Migration Plan'});expect(invoke.mock.calls.find(([c])=>c==='plan_type_migration')?.[1].input).toEqual({operation:'conversions',target:'Rarity',fromUnderlyingImplicit:true,toUnderlyingImplicit:true});expect(screen.queryByLabelText('Numeric value')).toBeNull();});
test('Custom Add passes exact initializer and field declaration to shared validation',async()=>{snapshot.category='Custom Type';snapshot.members=[];snapshot.fields=[{key:0,name:'amount',type:'ulong',nullable:false,array:false}];await open();fireEvent.click(screen.getByRole('button',{name:'Add Field',exact:true}));fireEvent.change(screen.getByLabelText('Field name'),{target:{value:'count'}});fireEvent.click(screen.getByRole('checkbox',{name:'Explicit constant initializer'}));fireEvent.change(screen.getByLabelText('Initializer (JSON value)'),{target:{value:'18446744073709551615'}});fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Type Migration Plan'});expect(invoke.mock.calls.find(([c])=>c==='plan_type_migration')?.[1].input.initializer).toBe('18446744073709551615');});
test('Drop confirmation is required and machine authorization is separate',async()=>{plan.destructive=true;plan.operation='DropEnumMember';await open();fireEvent.click(screen.getByRole('button',{name:'Drop Member',exact:true}));fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));const apply=await screen.findByRole('button',{name:'Apply reviewed Plan'});expect((apply as HTMLButtonElement).disabled).toBe(true);fireEvent.click(screen.getByRole('checkbox',{name:/I confirm dropping Rarity.Rare/}));fireEvent.click(apply);await waitFor(()=>expect(onResult).toHaveBeenCalledOnce());expect(invoke.mock.calls.find(([c])=>c==='apply_table_migration')?.[1].allowDestructive).toBe(true);});
test.each(['not_started','rolled_back','recovery_required'])('failure %s retains input and reports source state',async(state)=>{outcome={state,files:['data.yaml'],diagnostic:{code:'E-MIGRATION',message:state==='not_started'?'migration plan is stale':'commit failed'},fileStates:[{path:'data.yaml',state:'old'}],recoveryWorkspace:state==='recovery_required'?'/recovery':null};await open();await rename();fireEvent.click(screen.getByRole('button',{name:'Apply reviewed Plan'}));await waitFor(()=>expect(onResult).toHaveBeenCalledOnce());expect((screen.getByLabelText('Member name') as HTMLInputElement).value).toBe('Epic');expect(screen.getByRole('status')).toBeTruthy();if(state==='not_started')expect((screen.getByRole('button',{name:'Apply reviewed Plan'}) as HTMLButtonElement).disabled).toBe(true);});
test('a new selection hides old declaration and pending Plan response',async()=>{let resolvePlan:(v:any)=>void=()=>{};invoke.mockImplementation(async command=>{if(command==='open_type')return snapshot;if(command==='plan_type_migration')return new Promise(resolve=>{resolvePlan=resolve;});});const props={projectPath:'/project',canWrite:true,dirtyPaths:[],beginApply,onResult,endApply};const view=render(<TypeEditor {...props} path="one.yaml"/>);await screen.findByText('18446744073709551615');fireEvent.click(screen.getByRole('button',{name:'Rename Member',exact:true}));fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));snapshot={...base,name:'Second'};view.rerender(<TypeEditor {...props} path="two.yaml"/>);await screen.findByRole('heading',{name:/Second/});resolvePlan(plan);await waitFor(()=>expect(screen.queryByRole('region',{name:'Type Migration Plan'})).toBeNull());});
