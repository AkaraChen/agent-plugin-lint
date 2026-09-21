"""astra 的 JSON 语法、表示能力和重复键独立验收。"""
import pathlib as P,tempfile,subprocess,json
B=P.Path('target/debug/ap-lint').resolve();S='https://agent-plugins.org/schemas/1.0.0/';passed=[]
def check(label,manifest,code,predicate,mcp=None,args=()):
 with tempfile.TemporaryDirectory() as d:
  p=P.Path(d);(p/'plugin.json').write_text(manifest)
  if mcp is not None:(p/'mcp.json').write_text(mcp)
  q=subprocess.run([str(B),str(p),'--json',*args],capture_output=True,text=True,timeout=8);r=json.loads(q.stdout);assert q.returncode==code,(label,q.returncode,r);assert predicate(r),(label,r);passed.append(label)
def fs(r):return [f for p in r['plugins'] for f in p['findings']]
base=json.dumps({'$schema':S+'plugin.schema.json','name':'a'})
check('strict minimal',base,0,lambda r:True,args=('--strict',))
check('large number representation limit',base[:-1]+',"unknown":1e400}',2,lambda r:bool(r['errors']) and not any(f['ruleId']=='AP-MANIFEST-JSON' for f in fs(r)))
check('large MCP number representation limit',base,2,lambda r:bool(r['errors']) and not any(f['ruleId']=='AP-MCP-ENVELOPE' for f in fs(r)),mcp='{"$schema":"'+S+'mcp.schema.json","mcpServers":{"bad":1e400}}')
check('private marker ordinary unknown object',base[:-1]+',"unknown":{"$serde_json::private::Number":"1e400"}}',1,lambda r:all(f['radius']!='fatal' for f in fs(r)))
check('private marker is ordinary author key',base[:-1]+',"author":{"$serde_json::private::Number":"1e400"}}',1,lambda r:any(f['pointer']=='/author/$serde_json::private::Number' for f in fs(r)))
check('duplicate escaped name uses last value',base[:-1]+',"na\\u006de":"b"}',0,lambda r:r['plugins'][0]['name']=='b' and any(f['ruleId']=='AP-ADVICE-DUPLICATE-JSON-KEY' and f['radius']=='advisory' for f in fs(r)))
check('duplicate name strict',base[:-1]+',"name":"b"}',1,lambda r:any(f['ruleId']=='AP-ADVICE-DUPLICATE-JSON-KEY' for f in fs(r)),args=('--strict',))
check('JSON trailing comma really invalid',base[:-1]+',}',1,lambda r:any(f['ruleId']=='AP-MANIFEST-JSON' and f['radius']=='fatal' for f in fs(r)))
check('same spelling duplicate header only advice',base,0,lambda r:any(f['ruleId']=='AP-ADVICE-DUPLICATE-JSON-KEY' for f in fs(r)),mcp='{"$schema":"'+S+'mcp.schema.json","mcpServers":{"s":{"type":"sse","url":"https://example.com","headers":{"X-A":"a","X-A":"b"}}}}')
q=subprocess.run([str(B),'--mode','--json'],capture_output=True,text=True);r=json.loads(q.stdout);assert q.returncode==2 and r['errors'][0]['code']=='ARGUMENT';passed.append('missing option value keeps JSON mode')
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
