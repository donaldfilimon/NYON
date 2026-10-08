"""Execute actual inline IndexedDB adapters against a controllable transaction."""
import json
import pathlib
import re
import shutil
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
HARNESS = r"""
const assert = require('node:assert/strict');
const vm = require('node:vm');
(async()=>{
const source=SOURCE;
for(const adapter of ['living','selection']) {
 for(const scenario of ['healthy','stale','first','late','delete','abortThrows','staleAbortThrows']) {
  if(adapter==='selection' && scenario==='delete') continue;
  const initial=new Map([['generation-a','a'],['generation-b','b'],['head','3'],['old',new Uint8Array([9])]]);
  let durable=new Map(initial), provisional=new Map(initial), closed=0, writes=0, escaped=[], aborts=0;
  const original=new DOMException('injected quota','QuotaExceededError');
  const abortError=new DOMException('aborted','AbortError');
  let t;
  const db={objectStoreNames:{contains:()=>true},close(){closed++;},transaction(){
   t={error:null,objectStore(){return {
    get(key){const r={};queueMicrotask(()=>{r.result=initial.get(key);try{r.onsuccess();}catch(e){escaped.push(e);} });return r;},
    put(value,key){writes++;if((scenario==='first'||scenario==='abortThrows')&&writes===1 || scenario==='late'&&writes===(adapter==='living'?4:2))throw original;provisional.set(key,value);},
    delete(key){if(scenario==='delete')throw original;provisional.delete(key);}
   };},abort(){aborts++;provisional=new Map(initial);t.error=abortError;queueMicrotask(()=>{t.onabort?.();t.onerror?.();t.onabort?.();});if(scenario==='abortThrows'||scenario==='staleAbortThrows')throw abortError;}};
   queueMicrotask(()=>queueMicrotask(()=>{if(!aborts){durable=new Map(provisional);t.oncomplete?.();t.oncomplete?.();}}));return t;
  }};
  const context={Uint8Array,JSON,Promise,Error,indexedDB:{open(){const r={};queueMicrotask(()=>{r.result=db;r.onsuccess();});return r;}}};
  vm.createContext(context);vm.runInContext(source[adapter].replaceAll('export async function','async function'),context);
  const expected=JSON.stringify(scenario.startsWith('stale')?['different','b','3']:['a','b','3']);
  const p=adapter==='living'?context.livingCommit(expected,'generation-b','next','4',JSON.stringify([['payload',[1]],['other',[2]]]),JSON.stringify(['old'])):context.nyonSelectionCas(expected,'next','generation-b','4');
  let result,error;
  p.then(v=>result=v,e=>error=e);
  for(let i=0;i<20;i++)await Promise.resolve();
  assert.deepEqual(escaped,[],`${adapter}/${scenario}: escaped event callback`);
  assert.equal(closed,1,`${adapter}/${scenario}: db closed once`);
  if(scenario==='healthy'){assert.equal(result,true);assert.equal(durable.get('head'),'4');assert.equal(durable.get('generation-b'),'next');if(adapter==='living'){assert.deepEqual([...durable.get('payload')],[1]);assert.equal(durable.has('old'),false);}}
  else if(scenario==='stale'){assert.equal(result,false);assert.equal(error,undefined);assert.deepEqual(durable,initial);assert.equal(writes,0);}
  else {assert.equal(error,scenario==='staleAbortThrows'?abortError:original,`${adapter}/${scenario}: preserve original error identity`);assert.deepEqual(durable,initial);assert.equal(aborts,1);}
 }
}
console.log('13 actual-adapter scenarios passed');
})().catch(e=>{console.error(e);process.exitCode=1;});
"""


class IndexedDbCallbackErrors(unittest.TestCase):
    @unittest.skipUnless(shutil.which('node'), 'Node unavailable')
    def test_actual_inline_adapters(self):
        sources = {}
        for name, path in [('living', 'src/living_store/web.rs'), ('selection', 'src/library_selection/web.rs')]:
            text = (ROOT / path).read_text()
            sources[name] = re.search(r'inline_js\s*=\s*r#"(.*?)"#', text, re.S).group(1)
        result = subprocess.run(['node', '-e', HARNESS.replace('SOURCE', json.dumps(sources), 1)], capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('13 actual-adapter scenarios passed', result.stdout)


if __name__ == '__main__':
    unittest.main()
