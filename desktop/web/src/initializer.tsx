import {useState} from "react";
import {Button, Checkbox, Empty, Flex, Input, Pagination, Select, Space, Tag, Typography} from "antd";
import {DeleteOutlined, LeftOutlined, PlusOutlined, RightOutlined} from "@ant-design/icons";
import type {ConstantInput, Shape} from "./types";
const unset:ConstantInput={kind:"unset"};
type Step=string|number;
function element(shape:Shape):Shape{return {...shape,array:false,nullable:false};}
function child(shape:Shape,key:Step):Shape{return typeof key==="number"?element(shape):shape.fields!.find(([f])=>f.name===key)![1];}
function at(input:ConstantInput,path:Step[]):ConstantInput {
  if(!path.length)return input;
  const [key,...rest]=path;
  const next=typeof key==="number"&&input.kind==="sequence"?input.value[key]:typeof key==="string"&&input.kind==="mapping"?input.value.find(([name])=>name===key)?.[1]:undefined;
  return at(next??unset,rest);
}
function replace(input:ConstantInput,path:Step[],value:ConstantInput):ConstantInput {
  if(!path.length)return value;
  const [key,...rest]=path;
  if(typeof key==="number"&&input.kind==="sequence")return {...input,value:input.value.map((old,i)=>i===key?replace(old,rest,value):old)};
  if(typeof key==="string"&&input.kind==="mapping")return {...input,value:input.value.map(([name,old]):[string,ConstantInput]=>[name,name===key?replace(old,rest,value):old])};
  return input;
}
const summary=(v:ConstantInput)=>v.kind==="scalar"?v.text:v.kind==="unset"?"未入力":v.kind==="null"?"null":`${v.value.length} ${v.kind==="sequence"?"要素":"フィールド"}`;

// One level is mounted at a time, with sixteen children per page. The shared
// resolved shape chooses controls; Rust alone interprets and validates input.
export function TypedInitializer({shape,value,onChange,disabled=false}:{shape:Shape;value:ConstantInput;onChange:(v:ConstantInput)=>void;disabled?:boolean}) {
  const [path,setPath]=useState<Step[]>([]),[page,setPage]=useState(1);
  const current=at(value,path),resolved=path.reduce(child,shape);
  const set=(v:ConstantInput)=>onChange(replace(value,path,v));
  const open=(key:Step)=>{setPath([...path,key]);setPage(1);};
  const materialize=()=>set(resolved.array||resolved.category==="flags"?{kind:"sequence",value:[]}:resolved.category==="custom"?{kind:"mapping",value:resolved.fields!.map(([f])=>[f.name,unset])}:{kind:"scalar",text:""});
  const entries=current.kind==="sequence"?current.value.map((v,i)=>({key:i,label:String(i+1),value:v})):current.kind==="mapping"?current.value.map(([name,v])=>({key:name,label:name,value:v})):[];
  const visiblePage=Math.min(page,Math.max(1,Math.ceil(entries.length/16)));
  return <div className="typed-initializer" aria-label="型に合わせた初期値">
    <Flex gap={8} align="center" className="initializer-heading">
      {!!path.length&&<Button type="text" icon={<LeftOutlined aria-hidden="true"/>} aria-label="親の初期値に戻る" onClick={()=>{setPath(path.slice(0,-1));setPage(1);}}/>}
      <Typography.Text strong>{path.map(key=>typeof key==="number"?String(key+1):key).join(" / ")||"初期値"}</Typography.Text>
      <Tag>{resolved.typeName}{resolved.array?"[]":resolved.nullable?"?":""}</Tag>
      <Space className="initializer-actions"><Button type="text" disabled={disabled} onClick={()=>set(unset)}>未設定</Button>{resolved.nullable&&<Button type="text" disabled={disabled} onClick={()=>set({kind:"null"})}>nullを使用</Button>}</Space>
    </Flex>
    {current.kind==="unset"||current.kind==="null"?<Flex gap={12} align="center"><Typography.Text type="secondary">{summary(current)}</Typography.Text><Button disabled={disabled} onClick={materialize}>値を入力</Button></Flex>
    :resolved.category==="flags"&&!resolved.array&&current.kind==="sequence"?<Checkbox.Group disabled={disabled} value={current.value.filter(v=>v.kind==="scalar").map(v=>(v as {kind:"scalar";text:string}).text)} options={resolved.members??[]} onChange={symbols=>set({kind:"sequence",value:symbols.map(symbol=>({kind:"scalar",text:String(symbol)}))})}/>
    :current.kind==="scalar"?<>{resolved.category==="enum"||resolved.typeName==="bool"||resolved.underlying==="bool"?<Select aria-label="初期値" placeholder="値を選択" disabled={disabled} value={current.text||undefined} options={(resolved.category==="enum"?resolved.members??[]:["true","false"]).map(v=>({value:v,label:v}))} onChange={text=>set({kind:"scalar",text})}/>:<Input aria-label="初期値" disabled={disabled} value={current.text} onChange={e=>set({kind:"scalar",text:e.target.value})}/>}</>
    :<>
      {entries.length===0&&<Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="空の配列"/>}
      <div role="list" aria-label="初期値の要素">{entries.slice((visiblePage-1)*16,visiblePage*16).map(entry=><Flex key={entry.key} role="listitem" align="center" className="initializer-child"><Button type="text" disabled={disabled} onClick={()=>open(entry.key)}><Typography.Text>{entry.label}</Typography.Text><Typography.Text type="secondary">{summary(entry.value)}</Typography.Text><RightOutlined aria-hidden="true"/></Button>{current.kind==="sequence"&&<Button type="text" disabled={disabled} icon={<DeleteOutlined aria-hidden="true"/>} aria-label={`初期値の要素を削除: ${entry.label}`} onClick={()=>set({...current,value:current.value.filter((_,i)=>i!==entry.key)})}/>}</Flex>)}</div>
      {entries.length>16&&<Pagination size="small" simple current={visiblePage} pageSize={16} total={entries.length} onChange={setPage}/>}
      {resolved.array&&current.kind==="sequence"&&<Button icon={<PlusOutlined aria-hidden="true"/>} disabled={disabled} onClick={()=>{set({...current,value:[...current.value,unset]});setPage(Math.floor(current.value.length/16)+1);}}>要素を追加</Button>}
    </>}
  </div>;
}
