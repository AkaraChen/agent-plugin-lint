"""astra 黑盒 review；仅写临时夹具，不改语料。"""
import json, subprocess, tempfile, pathlib, sys
B=pathlib.Path(__file__).resolve().parents[2]/'target/debug/ap-lint'
S='https://agent-plugins.org/schemas/1.0.0/'
passed=[]
def run(root,*args):
 p=subprocess.run([str(B),str(root),'--mode','plugin','--json',*args],capture_output=True,text=True,timeout=10)
 try:r=json.loads(p.stdout)
 except Exception:raise AssertionError((p.returncode,p.stdout,p.stderr))
 assert r['exitCode']==p.returncode,(p.returncode,r)
 return p,r

def case(label,manifest,code,expect=None,extra=None,args=()):
 with tempfile.TemporaryDirectory() as d:
  root=pathlib.Path(d);(root/'plugin.json').write_text(json.dumps(manifest))
  if extra:extra(root)
  p,r=run(root,*args);assert p.returncode==code,(label,p.returncode,r)
  if expect:assert expect(r),(label,r)
  passed.append(label)

def fs(r):return [f for p in r['plugins'] for f in p['findings']]
def has(rule,radius=None,effect=None):return lambda r:any(f['ruleId']==rule and (radius is None or f['radius']==radius) and (effect is None or f['effect']==effect) for f in fs(r))
def base(**kw):return {'$schema':S+'plugin.schema.json','name':'a',**kw}
case('minimal',base(),0)
case('period name',base(name='acme.tools'),0)
case('mixed punctuation',base(name='a.-b'),0)
case('unknown field ignored',base(skills=['elsewhere']),1,has('AP-MANIFEST-UNKNOWN-FIELD','ignored','ignore-field'))
case('extensions nonobject ignored',base(extensions=[]),1,has('AP-EXTENSIONS-OBJECT','ignored','ignore-field'))
case('author nested closed',base(author={'github':'x'}),1,has('AP-MANIFEST-AUTHOR','fatal'))
case('metadata formats allowed',base(homepage='local',repository='x',license='custom',author={'email':'local','url':'x'}),0)
case('nonsemver advisory',base(version='nightly'),0,has('AP-VERSION-SEMVER','advisory'))
case('nonsemver strict',base(version='nightly'),1,args=('--strict',))
case('name repetition',base(name='a--b'),1,has('AP-NAME-REPETITION','fatal'))
case('manifest version unsupported',base(**{'$schema':'https://agent-plugins.org/schemas/1.1.0/plugin.schema.json'}),1,has('AP-MANIFEST-SCHEMA-ID','fatal'))
case('empty author allowed',base(author={}),0)
case('pointer escape',base(**{'a~/b':3}),1,lambda r:any(f['pointer']=='/a~0~1b' for f in fs(r)))
if len(sys.argv)>1 and sys.argv[1]=='full':
 def mcp(config):return lambda root:(root/'mcp.json').write_text(json.dumps({'$schema':S+'mcp.schema.json','mcpServers':config}))
 case('all transports',base(),0,extra=mcp({'s':{'type':'stdio','command':'node'},'h':{'type':'streamable-http','url':'https://example.com'},'e':{'type':'sse','url':'http://[::1]'}}))
 case('cross variant',base(),1,has('AP-MCP-SERVER-VARIANT','component','skip-server'),mcp({'s':{'type':'stdio','command':'node','url':'https://example.com'}}))
 case('userinfo empty',base(),1,has('AP-MCP-URL','component'),mcp({'h':{'type':'streamable-http','url':'https://@example.com'}}))
 case('fragment empty',base(),1,has('AP-MCP-URL','component'),mcp({'h':{'type':'streamable-http','url':'https://example.com/#'}}))
 case('nonloopback HTTP',base(),1,has('AP-MCP-HTTPS','component'),mcp({'h':{'type':'streamable-http','url':'http://example.com'}}))
 case('127/8 HTTP',base(),0,extra=mcp({'h':{'type':'streamable-http','url':'http://127.2.3.4'}}))
 case('reserved env',base(),1,has('AP-MCP-RESERVED-ENV','component'),mcp({'s':{'type':'stdio','command':'node','env':{'PLUGIN_ROOT':'/tmp'}}}))
 case('opaque values',base(),0,extra=mcp({'s':{'type':'stdio','command':'node','args':['../../outside','/tmp/x'],'env':{'OUTPUT':'/outside'}}}))
 case('header case duplicate',base(),1,has('AP-MCP-HEADERS','component'),mcp({'h':{'type':'sse','url':'https://example.com','headers':{'X-A':'a','x-a':'b'}}}))
 case('unknown extension nonobject value untouched',base(extensions={'com.example':3}),0)
 import os
 corpus=pathlib.Path(os.environ['AP_LINT_CORPUS'])
 p=subprocess.run([str(B),str(corpus),'--json'],capture_output=True,text=True,timeout=30);r=json.loads(p.stdout)
 assert p.returncode==1,(p.returncode,r)
 actual={(pr['root']+'/'+f['path'],f['radius'],f['effect']) for pr in r['plugins'] for f in pr['findings'] if f['ruleId'] in ['AP-PATH-RESOURCE-ESCAPE','AP-PATH-SKILL-ESCAPE']}
 expected={('frontend/skills/e2e-testing/references/docker.md','ignored','deny-path'),('review/skills/github-pr/references/viewed-state.md','ignored','deny-path'),('review/skills/guided-review','component','skip-skill')}
 assert actual==expected,(actual,expected);assert len(r['plugins'])==11
 p2=subprocess.run([str(B),str(corpus),'--json'],capture_output=True,text=True,timeout=30);assert p.stdout==p2.stdout
 passed.extend(['corpus exact 3 containment findings','corpus JSON deterministic'])
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
