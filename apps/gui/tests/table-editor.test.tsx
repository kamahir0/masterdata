import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import TableEditor from '../src/TableEditor';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
const snapshot={path:'schema.yaml',schema:{table:'item',fields:[{key:0,name:'id',type:'int',nullable:false,array:false},{key:1,name:'note',type:'string',nullable:false,array:false}],primaryKey:{fields:['id']},secondaryKeys:[]},fieldTypes:['int','string','ulong']};
const planned={token:'1',table:'item',operation:'RenameField',field:'note',destructive:false,affectedRecordCount:2,files:[{path:'data.yaml',before:'note: before',after:'description: before'}],diagnostics:[]};
let plan:any;let outcome:any;
const onResult=vi.fn(async()=>{});const beginApply=vi.fn(()=>true);const endApply=vi.fn();
beforeEach(()=>{plan=structuredClone(planned);outcome={state:'success',files:['data.yaml']};invoke.mockReset();onResult.mockClear();beginApply.mockClear();endApply.mockClear();invoke.mockImplementation(async command=>{
  if(command==='open_table')return snapshot;
  if(command==='plan_table_migration')return plan;
  if(command==='apply_table_migration')return outcome;
  throw Error(command);
});});
afterEach(cleanup);
async function open(dirtyPaths:string[]=[]) {render(<TableEditor projectPath="/project" path="schema.yaml" canWrite dirtyPaths={dirtyPaths} beginApply={beginApply} onResult={onResult} endApply={endApply}/>);await screen.findByText('note',{selector:'.ant-table-cell'});}
async function rename() {fireEvent.click(screen.getByRole('button',{name:'Rename Field',exact:true}));fireEvent.change(screen.getByLabelText('Field name'),{target:{value:'itemId'}});fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Migration Plan'});}
test('input changes invalidate the reviewed plan and Diff uses its captured source',async()=>{
  await open();await rename();expect(screen.getByText('note: before')).toBeTruthy();
  fireEvent.change(screen.getByLabelText('Field name'),{target:{value:'another'}});
  expect(screen.queryByRole('button',{name:'Apply reviewed Plan'})).toBeNull();
  expect(invoke.mock.calls.some(([command])=>command==='apply_table_migration')).toBe(false);
});
test('affected dirty files block Apply while unrelated dirty files do not',async()=>{
  await open(['data.yaml']);await rename();
  expect((screen.getByRole('button',{name:'Apply reviewed Plan'}) as HTMLButtonElement).disabled).toBe(true);
  cleanup();await open(['other.yaml']);await rename();
  fireEvent.click(screen.getByRole('button',{name:'Apply reviewed Plan'}));
  await waitFor(()=>expect(onResult).toHaveBeenCalledOnce());
  expect(invoke.mock.calls.find(([command])=>command==='apply_table_migration')?.[1]).toEqual({projectPath:'/project',token:'1',allowDestructive:false});
});
test('Drop requires explicit confirmation and passes destructive authorization separately',async()=>{
  plan.destructive=true;plan.operation='DropField';await open();
  fireEvent.click(screen.getByRole('button',{name:'Drop Field',exact:true}));fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));
  const apply=await screen.findByRole('button',{name:'Apply reviewed Plan'});
  expect((apply as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('checkbox',{name:/I confirm dropping/}));fireEvent.click(apply);
  await waitFor(()=>expect(onResult).toHaveBeenCalledOnce());
  expect(invoke.mock.calls.find(([command])=>command==='apply_table_migration')?.[1].allowDestructive).toBe(true);
});
test('stale preflight preserves inputs but prevents retrying the old Plan',async()=>{
  outcome={state:'not_started',files:[],diagnostic:{code:'E-MIGRATION-PATCH-INVALID',message:'migration plan is stale'}};
  await open();await rename();fireEvent.click(screen.getByRole('button',{name:'Apply reviewed Plan'}));
  await screen.findByText('Stale Plan — re-plan required');
  expect((screen.getByLabelText('Field name') as HTMLInputElement).value).toBe('itemId');
  expect((screen.getByRole('button',{name:'Apply reviewed Plan'}) as HTMLButtonElement).disabled).toBe(true);
});
test('Add initializer reaches shared service as exact text',async()=>{
  await open();fireEvent.click(screen.getByRole('button',{name:'Add Field',exact:true}));
  fireEvent.change(screen.getByLabelText('Field name'),{target:{value:'amount'}});
  fireEvent.click(screen.getByRole('checkbox',{name:'Explicit constant initializer'}));
  fireEvent.change(screen.getByLabelText('Initializer (JSON value)'),{target:{value:'18446744073709551615'}});
  fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Migration Plan'});
  expect(invoke.mock.calls.find(([command])=>command==='plan_table_migration')?.[1].input.initializer).toBe('18446744073709551615');
});
