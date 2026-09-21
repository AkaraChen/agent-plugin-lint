import pathlib as P,tempfile,subprocess,json,os
B=str(P.Path('target/debug/ap-lint').resolve());S='https://agent-plugins.org/schemas/1.0.0/plugin.schema.json'
passed=[]
with tempfile.TemporaryDirectory() as d:
 t=P.Path(d)
 def m(p):p.mkdir(parents=True,exist_ok=True);(p/'plugin.json').write_text(json.dumps({'$schema':S,'name':'a'}))
 def call(p,*a,cwd=None):
  q=subprocess.run([B,str(p),'--json',*a],cwd=cwd,capture_output=True,text=True,timeout=8);return q.returncode,json.loads(q.stdout)
 m(t/'a');(t/'a'/'locked').mkdir();(t/'a'/'locked'/'real').write_text('{}');(t/'a'/'plugin.json').unlink();(t/'a'/'plugin.json').symlink_to('locked/real');(t/'a'/'locked').chmod(0)
 try:
  code,r=call(t/'a','--mode','plugin');assert code==2 and r['errors'] and not r['complete'],r;assert not r['plugins'][0]['findings'];assert not any(c['status']=='fail' for c in r['plugins'][0]['coverage']),r;passed.append('permission errors are IO')
 finally:(t/'a'/'locked').chmod(0o700)
 p=t/'missing';p.mkdir();code,r=call(p,'--mode','plugin');c=r['plugins'][0]['coverage'];assert any(x['ruleId']=='AP-MANIFEST-LOCATION' and x['status']=='fail' for x in c);assert all(any(x['ruleId']==k and x['status']=='blocked' for x in c) for k in ['AP-SKILL-CONFORMANCE','AP-MCP-ENVELOPE']);passed.append('missing manifest coverage')
 m(t/'collection'/'one');code,r=call(t/'collection');assert r['plugins'][0]['root']=='one',r;passed.append('relative collection root')
 m(t/'cwd');(t/'cwd'/'plugin.json').write_text('{');code,r=call(t/'collection','--bad',cwd=t/'cwd');assert r['summary']=={'fatal':0,'component':0,'ignored':0,'advisory':0,'errors':1},r;passed.append('argument error clean summary')
 p=os.fsencode(t)+b'/bad\xff';os.mkdir(p);open(p+b'/plugin.json','wb').write(json.dumps({'$schema':S,'name':'a'}).encode());q=subprocess.run([os.fsencode(B),p,b'--mode',b'plugin',b'--json'],capture_output=True);r=json.loads(q.stdout);assert q.returncode==2 and any(e['code']=='PATH_ENCODING' for e in r['errors']),r;passed.append('nonUTF8 explicit encoding error')
 for a in [('--mode','plugin','--mode','auto'),('--spec','1.0.0','--spec','1.0.0')]:
  code,r=call(t/'collection'/'one',*a);assert code==2 and any(e['code']=='ARGUMENT' for e in r['errors']),r
 passed.append('duplicate options rejected')
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
