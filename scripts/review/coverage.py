"""astra 的全表覆盖、strict 分界和文本报告独立验收。"""
import pathlib as P,tempfile,subprocess,json,re
B=P.Path('target/debug/ap-lint').resolve();S='https://agent-plugins.org/schemas/1.0.0/';passed=[]
rows={}
for line in P.Path('rules.md').read_text().splitlines():
 if re.match(r'\| (?:AP|AS)-[A-Z-]+ \|',line):
  parts=[x.strip() for x in line.split('|')];rows[parts[1]]=parts[4]
assert len(rows)==91,len(rows)
with tempfile.TemporaryDirectory() as d:
 p=P.Path(d)
 def run(label,manifest=None,mcp=None,args=(),code=0):
  (p/'plugin.json').write_text(json.dumps(manifest or {'$schema':S+'plugin.schema.json','name':'a'}))
  f=p/'mcp.json'
  if f.exists():f.unlink()
  if mcp is not None:f.write_text(json.dumps({'$schema':S+'mcp.schema.json','mcpServers':mcp}))
  q=subprocess.run([str(B),str(p),'--json',*args],capture_output=True,text=True,timeout=8);r=json.loads(q.stdout)
  assert q.returncode==code,(label,q.returncode,r)
  coverage=r['plugins'][0]['coverage'];assert set(rows)<=set(x['ruleId'] for x in coverage),(label,set(rows)-set(x['ruleId'] for x in coverage))
  for c in coverage:
   if c['ruleId'] in rows and any(x in rows[c['ruleId']] for x in ['C/T','U/M']):assert c['status'] not in ['pass','fail'],(label,c)
  passed.append(label);return r
 r=run('minimal full registry strict',args=('--strict',))
 assert any(c['status'] in ['manual','runtime'] for c in r['plugins'][0]['coverage'])
 run('ambiguous IP default',mcp={'s':{'type':'sse','url':'http://127.1'}})
 run('ambiguous IP strict',mcp={'s':{'type':'sse','url':'http://127.1'}},args=('--strict',),code=1)
 run('DATA runtime does not fail strict',mcp={'s':{'type':'stdio','command':'node','cwd':'${PLUGIN_DATA}/future'}},args=('--strict',))
 r=run('fatal manifest retains full coverage',manifest={'$schema':S+'plugin.schema.json','name':'BAD'},code=1)
 assert not any(c['status']=='pass' and c['ruleId'].startswith(('AS-','AP-MCP-')) for c in r['plugins'][0]['coverage'])
 r=run('license needs manual assessment',manifest={'$schema':S+'plugin.schema.json','name':'a','license':'MIT'},args=('--strict',))
 assert any(c['ruleId']=='AP-LICENSE-SPDX' and c['status']=='manual' for c in r['plugins'][0]['coverage'])
 r=run('stdio author intent stays manual',mcp={'s':{'type':'stdio','command':'node'}},args=('--strict',))
 for id in ['AP-MCP-BUNDLED-COMMAND','AP-MCP-PATH-DEPENDENCE','AP-ENV-BASE-DEPENDENCE']:
  assert any(c['ruleId']==id and c['status'] in ['manual','runtime'] for c in r['plugins'][0]['coverage']),(id,r)
 r=run('unknown namespace values manual',manifest={'$schema':S+'plugin.schema.json','name':'a','extensions':{'com.example':3}})
 assert any(c['ruleId']=='AP-EXTENSION-VALUE' and c['status']=='manual' for c in r['plugins'][0]['coverage'])
 assert not any(f['ruleId']=='AP-EXTENSION-UNKNOWN' for f in r['plugins'][0]['findings'])
 # 文本含实际路径、证据、§，以及不等于通过的覆盖状态。
 run('text fixture',manifest={'$schema':S+'plugin.schema.json','name':'BAD'},code=1)
 q=subprocess.run([str(B),str(p)],capture_output=True,text=True)
 assert all(x in q.stdout for x in ['plugin.json','AP-NAME-CHARSET','§5.5']),q.stdout
 assert any(x in q.stdout for x in ['blocked','未检查','阻断']),q.stdout
 passed.append('text actionable and incomplete coverage visible')
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
