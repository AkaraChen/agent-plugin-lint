"""astra 的扩展边界独立验收；未知 namespace 的值与文件不解释。"""
import pathlib as P,tempfile,json,subprocess,os
B=P.Path('target/debug/ap-lint').resolve();S='https://agent-plugins.org/schemas/1.0.0/plugin.schema.json';passed=[]
def case(label,extensions,code,check=lambda r:True,setup=None,omit=False):
 with tempfile.TemporaryDirectory() as d:
  p=P.Path(d);m={'$schema':S,'name':'a'}
  if not omit:m['extensions']=extensions
  (p/'plugin.json').write_text(json.dumps(m))
  if setup:setup(p)
  q=subprocess.run([str(B),str(p),'--json'],capture_output=True,text=True,timeout=8);r=json.loads(q.stdout);assert q.returncode==code,(label,q.returncode,r);assert check(r),(label,r);passed.append(label)
def fs(r):return r['plugins'][0]['findings']
case('unknown primitive value ignored',{'com.example':3},0,lambda r:not fs(r))
case('unknown null value ignored',{'com.example':None},0,lambda r:not fs(r))
case('unknown array value ignored',{'com.example':[1,{'anything':False}]},0,lambda r:not fs(r))
case('manifest only namespace',{'com.example':{}},0)
case('directory only namespace',None,0,setup=lambda p:(p/'com.example').mkdir(),omit=True)
case('unknown extension content not read',{'com.example':{}},0,setup=lambda p:((p/'com.example').mkdir(),os.mkfifo(p/'com.example'/'config')))
case('empty namespace invalid',{'':{}},1,lambda r:any(f['ruleId']=='AP-EXTENSION-NAMESPACE' and f['radius']=='fatal' for f in fs(r)))
case('path namespace invalid',{'../x':{}},1,lambda r:any(f['pointer']=='/extensions/..~1x' for f in fs(r)))
case('invalid namespace blocks MCP',{'':{}},1,lambda r:not r['errors'] and not any(f['ruleId']=='AP-DISCOVERY-KIND' for f in fs(r)),setup=lambda p:os.mkfifo(p/'mcp.json'))
case('ambiguous namespace unchecked',{'example':{}},0,lambda r:any(c['ruleId']=='AP-EXTENSION-NAMESPACE' and c['status']=='unchecked' for c in r['plugins'][0]['coverage']))
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
