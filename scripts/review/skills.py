import pathlib as P,tempfile,subprocess,json,os
B=P.Path('target/debug/ap-lint').resolve();S='https://agent-plugins.org/schemas/1.0.0/plugin.schema.json';passed=[]
with tempfile.TemporaryDirectory() as d:
 t=P.Path(d)
 def plugin(n):p=t/n;p.mkdir();(p/'plugin.json').write_text(json.dumps({'$schema':S,'name':'a'}));return p
 def skill(p,body='ok',front='name: s\ndescription: ok'):
  s=p/'skills'/'s';s.mkdir(parents=True);(s/'SKILL.md').write_text('---\n'+front+'\n---\n'+body);return s
 def run(p,code):
  q=subprocess.run([str(B),str(p),'--json'],capture_output=True,text=True,timeout=8);r=json.loads(q.stdout);assert q.returncode==code,(p,code,q.returncode,r);assert r['complete']==(code!=2),r;return r
 def fs(r):return r['plugins'][0]['findings']
 def cs(r):return r['plugins'][0]['coverage']
 p=plugin('permissions');(p/'skills').mkdir();(p/'skills').chmod(0)
 try:r=run(p,2);assert r['errors'] and not fs(r);passed.append('unreadable skills IO')
 finally:(p/'skills').chmod(0o700)
 p=plugin('fixed-lock');(p/'locked').mkdir();(p/'locked'/'inner').mkdir();(p/'skills').symlink_to('locked/inner');(p/'locked').chmod(0)
 try:r=run(p,2);assert not fs(r);passed.append('fixed permission not kind violation')
 finally:(p/'locked').chmod(0o700)
 p=plugin('fake-skill');(p/'skills').mkdir();out=t/'outside';(out/'SKILL.md').mkdir(parents=True);(p/'skills'/'helper').symlink_to(out);r=run(p,1);assert not any(f['effect']=='skip-skill' for f in fs(r));passed.append('external SKILL directory not skill')
 p=plugin('fake-md');s=skill(p);(s/'SKILL.md').unlink();(s/'SKILL.md').symlink_to(out/'SKILL.md');r=run(p,1);assert not any(f['effect']=='skip-skill' for f in fs(r));passed.append('external SKILL.md directory target not skill')
 p=plugin('large');skill(p,'x\n'*500);r=run(p,0);assert any(f['ruleId']=='AS-SIZE-GUIDANCE' and f['radius']=='advisory' for f in fs(r));assert not any(c['ruleId']=='AP-SKILL-CONFORMANCE' and c['status']=='fail' for c in cs(r));assert any(c['ruleId']=='AS-SIZE-GUIDANCE' and c['status']=='manual' for c in cs(r));passed.append('size advisory retained')
 p=plugin('valid');skill(p);r=run(p,0);c=[x for x in cs(r) if x['target']=='skills/s/SKILL.md'];assert {'AS-FRONTMATTER','AS-NAME','AS-DESCRIPTION','AS-OPTIONAL-FIELDS'} <= {x['ruleId'] for x in c};assert any(x['status']=='manual' for x in c);assert not any(x.get('reasonCode')=='NOT_IMPLEMENTED_S2B' for x in cs(r));passed.append('full AS coverage retained')
 p=plugin('bad-front');s=skill(p);(s/'SKILL.md').write_text('body only');r=run(p,1);assert any(f['ruleId']=='AS-FRONTMATTER' for f in fs(r));assert any(c['ruleId']=='AS-NAME' and c['status']=='blocked' for c in cs(r));assert not any(f['ruleId']=='AP-SKILL-CONFORMANCE' for f in fs(r));assert not any(c['ruleId']=='AP-SKILL-CONFORMANCE' and c['status']=='pass' for c in cs(r));passed.append('frontmatter error AS ID and blocked children')
 p=plugin('unknown-front');skill(p,front='name: s\ndescription: ok\ncustom-field: yes');r=run(p,0);assert any(c['ruleId']=='AP-SKILL-CONFORMANCE' and c['status']=='unchecked' for c in cs(r));assert any(f['ruleId']=='AP-ADVICE-SKILLS-UNCHECKED' for f in fs(r));passed.append('unchecked AS not aggregate pass')
 p=plugin('broken');s=skill(p);(s/'broken').symlink_to('missing');r=run(p,0);assert any(f['ruleId']=='AP-ADVICE-UNRESOLVED-PATH' and f['path']=='skills/s/broken' for f in fs(r));passed.append('broken resource visible unchecked')
 p=plugin('resource-scope');s=skill(p);q=t/'outside-doc';q.write_text('x');(s/'ref').symlink_to(q);r=run(p,1);assert any(f['path']=='skills/s/ref' and f['scope']['id']=='skills/s/ref' for f in fs(r));passed.append('resource scope exact path')
 p=plugin('nonutf8-resource');s=skill(p);os.mkdir(os.fsencode(s)+b'/bad\xff');r=run(p,2);assert any(e['code']=='PATH_ENCODING' for e in r['errors']);passed.append('nonUTF8 resource explicit')
 p=plugin('helper-without-skill');(p/'skills').mkdir();q=t/'external-helper';q.mkdir();(p/'skills'/'helper').symlink_to(q);r=run(p,1);assert not any(f['effect']=='skip-skill' for f in fs(r));passed.append('external helper missing SKILL not IO')
 p=plugin('directory-named-skill-md');(p/'skills').mkdir();q=t/'external-named-skill';q.mkdir();(q/'SKILL.md').write_text('outside');(p/'skills'/'SKILL.md').symlink_to(q);r=run(p,1);assert any(f['effect']=='skip-skill' for f in fs(r));passed.append('candidate basename not path role')
 p=plugin('wrong-case');s=skill(p);(s/'SKILL.md').rename(s/'skill.md');r=run(p,0);assert not any(c['ruleId'].startswith('AS-') and c['status']=='pass' for c in cs(r));passed.append('exact SKILL filename')
 p=plugin('multiple-name-errors');skill(p,front='name: BAD--NAME\ndescription: ok');r=run(p,1);assert sum(c['ruleId']=='AS-NAME' and c['target']=='skills/s/SKILL.md' for c in cs(r))==1;passed.append('coverage unique per AS rule and file')
 p=plugin('mixed-error-unknown');skill(p,front='name: BAD\ndescription: ok');q=p/'skills'/'unknown';q.mkdir();(q/'SKILL.md').write_text('---\nname: unknown\ndescription: ok\ncustom: yes\n---\n');r=run(p,1);assert sum(f['ruleId']=='AP-ADVICE-SKILLS-UNCHECKED' for f in fs(r))==1;passed.append('unchecked warning survives other errors')
 p=plugin('mixed-io-valid');skill(p);q=p/'skills'/'locked';q.mkdir();f=q/'SKILL.md';f.write_text('---\nname: locked\ndescription: ok\n---\n');f.chmod(0)
 try:r=run(p,2);assert not any(c['ruleId']=='AP-SKILL-CONFORMANCE' and c['target']=='skills' and c['status']=='pass' for c in cs(r));passed.append('read failure not hidden by good skill')
 finally:f.chmod(0o600)
 p=plugin('missing-components');r=run(p,0);assert all(any(c['ruleId']==k and c['status']=='not-applicable' for c in cs(r)) for k in ['AP-SKILL-CONFORMANCE','AP-MCP-ENVELOPE']);passed.append('missing components not applicable')
print(json.dumps({'passed':len(passed),'cases':passed},ensure_ascii=False,indent=2))
