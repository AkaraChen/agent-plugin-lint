"""实际扫描证据与 coverage 一致性；不以完整注册表代替真实执行。"""
import pathlib as P,tempfile,subprocess,json
B=P.Path('target/debug/ap-lint').resolve();S='https://agent-plugins.org/schemas/1.0.0/';passed=[];failures=[]
def check(label,prepare,expect):
 with tempfile.TemporaryDirectory() as d:
  p=P.Path(d)/'plugin';p.mkdir();(p/'plugin.json').write_text(json.dumps({'$schema':S+'plugin.schema.json','name':'a'}));prepare(p)
  q=subprocess.run([str(B),str(p),'--json'],capture_output=True,text=True,timeout=8);r=json.loads(q.stdout);c=r['plugins'][0]['coverage'];f=r['plugins'][0]['findings']
  try:
   keys=[(x['ruleId'],x['target']) for x in c];assert len(keys)==len(set(keys)),"同一rule/target重复coverage"
   expect(c,f,r);passed.append(label)
  except AssertionError as e:failures.append({'case':label,'reason':str(e)})
def status(c,id,allowed,target=None):
 x=[v for v in c if v['ruleId']==id and (target is None or v['target']==target)];assert x and any(v['status'] in allowed for v in x),(id,x)
def minimal(c,f,r):
 for id in ['AP-MANIFEST-LOCATION','AP-PATH-MANIFEST-ESCAPE']:status(c,id,['pass'])
check('existing manifest has actual location and containment evidence',lambda p:None,minimal)
def mcp(p,value): (p/'mcp.json').write_text(json.dumps({'$schema':S+'mcp.schema.json','mcpServers':value}))
def valid_mcp(c,f,r):
 for id in ['AP-DISCOVERY-KIND','AP-PATH-FIXED-ESCAPE']:status(c,id,['pass'],'mcp.json')
check('valid MCP kind and containment are checked',lambda p:mcp(p,{'s':{'type':'stdio','command':'node'}}),valid_mcp)
def all_findings_have_failure(c,f,r):
 assert f
 for x in f:status(c,x['ruleId'],['fail'])
check('invalid fixed component finding has fail coverage',lambda p:(p/'skills').write_text('wrong kind'),all_findings_have_failure)
def skill(p,source):
 s=p/'skills'/'a';s.mkdir(parents=True);(s/'SKILL.md').write_text(source);return s
def resource(p):
 s=skill(p,'---\nname: a\ndescription: Valid\n---\nBody\n');outside=p.parent/'outside';outside.write_text('external');(s/'ref').symlink_to(outside)
check('resource escape finding has fail coverage',resource,all_findings_have_failure)
check('read skill description quality remains manual',lambda p:skill(p,'---\nname: a\ndescription: Valid\n---\nBody\n'),lambda c,f,r:status(c,'AS-DESCRIPTION-QUALITY',['manual']))
def limited(p): (p/'plugin.json').write_text('{"$schema":"'+S+'plugin.schema.json","name":"a","unknown":1e400}')
def blocked(c,f,r):
 assert r['exitCode']==2
 for id in ['AS-DESCRIPTION-QUALITY','AP-LICENSE-SPDX','AP-MCP-COMMAND']:status(c,id,['blocked'])
check('representation error blocks unknown downstream targets',limited,blocked)
def broken_mcp(p):(p/'mcp.json').write_text('{')
def mcp_blocked(c,f,r):
 for id in ['AP-MCP-BUNDLED-COMMAND','AP-MCP-ENV-SECRETS','AP-MCP-COMMAND']:
  status(c,id,['blocked']);assert all(x['status']=='blocked' for x in c if x['ruleId']==id),(id,[x for x in c if x['ruleId']==id])
check('bad MCP envelope blocks downstream assessment',broken_mcp,mcp_blocked)
def ignored_extensions(p): (p/'plugin.json').write_text(json.dumps({'$schema':S+'plugin.schema.json','name':'a','extensions':3}))
check('ignored extension field blocks value inspection',ignored_extensions,lambda c,f,r:status(c,'AP-EXTENSION-VALUE',['blocked']))
def relative(p):
 (p/'runner').write_text('no execution');mcp(p,{'s':{'type':'stdio','command':'./runner'}})
check('relative form actually checked',relative,lambda c,f,r:status(c,'AP-PATH-RELATIVE-FORM',['pass']))
def safe_skill(c,f,r):
 for id in ['AP-PATH-FIXED-ESCAPE','AP-DISCOVERY-KIND']:status(c,id,['pass'],'skills')
 status(c,'AP-PATH-SKILL-ESCAPE',['pass'])
check('safe skill containment actually checked',lambda p:skill(p,'---\nname: a\ndescription: Valid\n---\nBody\n'),safe_skill)
print(json.dumps({'passed':len(passed),'cases':passed,'failures':failures},ensure_ascii=False,indent=2));assert not failures
