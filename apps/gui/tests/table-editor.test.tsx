import React from 'react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import TableEditor from '../src/TableEditor';
const {invoke}=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke}));
const snapshot={path:'schema.yaml',schema:{table:'item',fields:[{key:0,name:'id',type:'int',nullable:false,array:false},{key:1,name:'note',type:'string',nullable:false,array:false}],primaryKey:{fields:['id']},secondaryKeys:[]},fieldTypes:['int','string','ulong'],initializerShapes:{int:{kind:'primitive',primitive:'int'},string:{kind:'primitive',primitive:'string'},ulong:{kind:'primitive',primitive:'ulong'}}};
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
test('affected dirty file disables Apply',async()=>{
  await open(['data.yaml']);await rename();
  expect((screen.getByRole('button',{name:'Apply reviewed Plan'}) as HTMLButtonElement).disabled).toBe(true);
});
test('unrelated dirty file allows Apply',async()=>{
  await open(['other.yaml']);await rename();
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
test('Reference authoring sends ordered declaration to the shared mutation boundary',async()=>{
  await open();
  fireEvent.click(screen.getByRole('button',{name:'Add Reference',exact:true}));
  fireEvent.change(screen.getByLabelText('Reference name'),{target:{value:'category'}});
  fireEvent.change(screen.getByLabelText('C# helper name (optional, exact)'),{target:{value:'GetCategoryMaster'}});
  fireEvent.change(screen.getByLabelText('Source fields (ordered, comma-separated)'),{target:{value:'regionId, categoryId'}});
  fireEvent.change(screen.getByLabelText('Target table'),{target:{value:'item-category'}});
  fireEvent.change(screen.getByLabelText('Target key fields (ordered, comma-separated)'),{target:{value:'regionId, id'}});
  fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));
  await screen.findByRole('region',{name:'Migration Plan'});
  expect(invoke.mock.calls.find(([command])=>command==='plan_table_migration')?.[1].input).toEqual({
    operation:'add_reference',table:'item',reference:{name:'category',csharpName:'GetCategoryMaster',fields:['regionId','categoryId'],target:{table:'item-category',fields:['regionId','id']}}
  });
});

test('changing field shape clears typed initializer and reviewed plan',async()=>{
  await open();fireEvent.click(screen.getByRole('button',{name:'Add Field',exact:true}));
  fireEvent.click(screen.getByRole('checkbox',{name:'Explicit constant initializer'}));
  fireEvent.change(screen.getByLabelText('Initializer'),{target:{value:'42'}});
  fireEvent.click(screen.getByRole('button',{name:'Plan / Re-plan'}));await screen.findByRole('region',{name:'Migration Plan'});
  fireEvent.mouseDown(screen.getByRole('combobox',{name:'Field type'}));
  fireEvent.click((await screen.findAllByText('string',{selector:'.ant-select-item-option-content'})).at(-1)!);
  expect(screen.queryByRole('region',{name:'Migration Plan'})).toBeNull();
  expect((screen.getByRole('checkbox',{name:'Explicit constant initializer'}) as HTMLInputElement).checked).toBe(false);
});
