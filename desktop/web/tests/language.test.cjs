const {readFileSync}=require('node:fs');
const {resolve}=require('node:path');
const {runInNewContext}=require('node:vm');
const {test}=require('node:test');
const assert=require('node:assert/strict');
const ts=require('typescript');
const source=readFileSync(resolve(__dirname,'../src/language.ts'),'utf8');
const moduleExports={};
runInNewContext(ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText,{exports:moduleExports});
const {uiMessage,actionLabel,statusLabel,complexSummary,sourceCategoryLabel,uiTerms}=moduleExports;

test('診断の固定文言とtechnical identityを区別する',()=>{
  assert.equal(uiMessage('E-CONFIG-PROFILE: invalid profile production'),'E-CONFIG-PROFILE: 無効なプロファイル: production');
  assert.equal(uiMessage('outside ulong range'),'ulongの範囲を超えています');
  assert.equal(uiMessage('unknown Rarity member Rare'),'RarityにメンバーRareはありません');
  // The more general unknown-type/member message must not consume Flags text.
  assert.equal(uiMessage('unknown / duplicate Flags member Debug'),'FlagsのメンバーDebugが不明、または重複しています');
  for(const text of ['MasterData','sources/item.yaml','Reward.ItemId','9223372036854775807','18446744073709551615','-9223372036854775808','external tool detail'])assert.equal(uiMessage(text),text);
});
test('操作IDは表示時にだけ日本語化する',()=>{
  assert.equal(actionLabel('Save'),'保存');assert.equal(actionLabel('Cancel'),'キャンセル');
  assert.equal(statusLabel('OutcomeUnknown'),'結果を確認できません');
  assert.equal(actionLabel('custom-action'),'custom-action');
});
test('構造の要約だけを翻訳し同形のscalar値を保持する',()=>{
  // Rust omits complex payloads from the bounded grid projection.
  assert.equal(complexSummary('[3 items]',true),'[3 要素]');
  assert.equal(complexSummary('(missing)',true),'（未指定）');
  assert.equal(complexSummary('(missing)',false),'(missing)');
  assert.equal(complexSummary('{2 fields}',true),'{2 フィールド}');
  assert.equal(complexSummary('[3 items]',false),'[3 items]');
  assert.equal(complexSummary('{2 fields}',false),'{2 fields}');
});

// GUI-SHELL-LANGUAGE-001: developer category labels and prose share terminology.
test('開発者のカテゴリ名と日本語の説明を使い分ける',()=>{
  assert.equal(sourceCategoryLabel('valueObject'),'Value Object');
  assert.equal(sourceCategoryLabel('enum'),'Enum');
  assert.equal(sourceCategoryLabel('flags'),'Flags');
  assert.equal(sourceCategoryLabel('custom'),'Custom Type');
  assert.equal(sourceCategoryLabel('table'),'テーブル');
  assert.equal(sourceCategoryLabel('future-category'),'future-category');
  assert.equal(uiTerms.build,'Build'); assert.equal(uiTerms.publish,'Publish');
  assert.equal(uiMessage('symbolic enum member required'),'Enumのメンバー名を指定してください');
  assert.equal(uiMessage('Custom requires a mapping'),'Custom Typeにはマッピングが必要です');
  assert.equal(uiMessage('Array requires a sequence'),'配列にはシーケンスが必要です');
  assert.equal(uiMessage('unknown / duplicate Flags member フラグ列挙型'),'Flagsのメンバーフラグ列挙型が不明、または重複しています');
});
