"""astra 独立文件系统/CLI 黑盒；只在临时目录创建夹具。"""
import json, pathlib as P, tempfile, subprocess, os, sys
B=P.Path(__file__).resolve().parents[2]/'target/debug/ap-lint'
S='https://agent-plugins.org/schemas/1.0.0/plugin.schema.json'
passed=[]
def manifest(p):
 p.mkdir(parents=True,exist_ok=True);(p/'plugin.json').write_text(json.dumps({'$schema':S,'name':'a'}))
def run(p,code,*args):
 q=subprocess.run([str(B),str(p),'--json',*args],capture_output=True,text=True,timeout=8)
 r=json.loads(q.stdout);assert q.returncode==code,(p,args,q.returncode,r);assert r['exitCode']==code
 assert r['complete']==(code!=2),(p,r)
 return r
with tempfile.TemporaryDirectory() as d:
 t=P.Path(d)
 manifest(t/'args-valid')
 for label,args in [('unknown',('--what',)),('fix',('--fix',)),('mode conflict',('--mode','plugin','--mode','collection')),('format conflict',('--format','text')),('spec unsupported',('--spec','1.1.0'))]:
  r=run(t/'args-valid',2,*args);assert any(e['code']=='ARGUMENT' for e in r['errors']);passed.append(label)
 p=t/'missing';p.mkdir();r=run(p,1,'--mode','plugin');assert any(f['ruleId']=='AP-MANIFEST-LOCATION' for f in r['plugins'][0]['findings']);passed.append('explicit missing manifest')
 p=t/'mixed';manifest(p/'a');(p/'b').mkdir();run(p,2);r=run(p,1,'--mode','collection');assert len(r['plugins'])==2;passed.append('auto mixed vs explicit collection')
 p=t/'empty';p.mkdir();run(p,2,'--mode','collection');passed.append('empty collection')
 p=t/'outside';manifest(p);q=t/'external';q.write_text('{"secret":"should not read"}');(p/'plugin.json').unlink();(p/'plugin.json').symlink_to(q);r=run(p,1);f=r['plugins'][0]['findings'];assert any(x['ruleId']=='AP-PATH-MANIFEST-ESCAPE' and x['radius']=='fatal' for x in f);assert 'secret' not in json.dumps(r);passed.append('outside manifest gate')
 p=t/'internal';manifest(p);(p/'plugin.json').rename(p/'actual');(p/'plugin.json').symlink_to('actual');run(p,0);passed.append('internal manifest link')
 p=t/'root-link';p.symlink_to(t/'internal',target_is_directory=True);run(p,0);passed.append('root link')
 p=t/'fifo';p.mkdir();os.mkfifo(p/'plugin.json');run(p,1);passed.append('FIFO manifest no hang')
 p=t/'broken';p.mkdir();(p/'plugin.json').symlink_to('missing');run(p,1);passed.append('broken manifest marker')
 p=t/'aliases';manifest(p/'real');(p/'alias').symlink_to('real',target_is_directory=True);run(p,2);passed.append('canonical root alias ambiguity')
 p=t/'fatal';p.mkdir();(p/'plugin.json').write_text('{');os.mkfifo(p/'mcp.json');(p/'skills').symlink_to('/root',target_is_directory=True);r=run(p,1);assert len(r['plugins'][0]['findings'])==1;passed.append('fatal manifest blocks components')
 if 'full' in sys.argv:
  def skill(p,name='s'):
   p.mkdir(parents=True);(p/'SKILL.md').write_text('---\nname: '+name+'\ndescription: test\n---\nbody\n')
  def findings(r):return [f for p in r['plugins'] for f in p['findings']]
  p=t/'skills-out';manifest(p);skill(t/'outside-skills'/'s');(p/'skills').symlink_to(t/'outside-skills',target_is_directory=True);r=run(p,1);assert any(f['effect']=='disable-type' for f in findings(r));passed.append('fixed skills outside disables type')
  p=t/'file-out';manifest(p);skill(p/'skills'/'s');(p/'skills'/'s'/'SKILL.md').unlink();(p/'skills'/'s'/'SKILL.md').symlink_to(t/'outside-skills'/'s'/'SKILL.md');r=run(p,1);assert any(f['ruleId']=='AP-PATH-SKILL-ESCAPE' and f['effect']=='skip-skill' for f in findings(r));passed.append('SKILL.md outside skips skill')
  p=t/'safe-links';manifest(p);skill(p/'skills'/'s');(p/'skills'/'s'/'loop').symlink_to('.',target_is_directory=True);(p/'skills'/'s'/'ref').symlink_to('SKILL.md');run(p,0);passed.append('safe links and cycle')
  p=t/'prefix';manifest(p);skill(p/'skills'/'s');q=t/'prefix-evil';q.mkdir();(q/'doc').write_text('outside');(p/'skills'/'s'/'doc').symlink_to(q/'doc');r=run(p,1);assert any(f['ruleId']=='AP-PATH-RESOURCE-ESCAPE' and f['effect']=='deny-path' for f in findings(r));passed.append('sibling prefix is outside')
  p=t/'raw-parent';manifest(p);skill(p/'skills'/'s');q=t/'raw-out';(q/'deep').mkdir(parents=True);(q/'target').write_text('outside');(p/'target').write_text('inside');(p/'bridge').symlink_to(q/'deep',target_is_directory=True);(p/'skills'/'s'/'ref').symlink_to('../../bridge/../target');assert (p/'skills'/'s'/'ref').read_text()=='outside';r=run(p,1);assert any(f['ruleId']=='AP-PATH-RESOURCE-ESCAPE' and f['path']=='skills/s/ref' for f in findings(r));passed.append('raw symlink parent follows kernel')
  p=t/'nonrecursive';manifest(p);skill(p/'skills'/'group'/'s');run(p,0);passed.append('no recursive skill discovery')
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
