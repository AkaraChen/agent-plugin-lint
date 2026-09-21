"""Linux 运行期观察：独立二进制、零网络系统调用、不执行被检命令。"""
import pathlib as P,tempfile,subprocess,json,shutil,os,shlex,hashlib
B=P.Path('target/debug/ap-lint').resolve();S='https://agent-plugins.org/schemas/1.0.0/'
assert shutil.which('strace'),'此验收需要 strace；缺少它不能记为通过'
with tempfile.TemporaryDirectory() as d:
 t=P.Path(d);binary=t/'ap-lint';shutil.copy2(B,binary);p=t/'plugin';p.mkdir()
 manifest={'$schema':S+'plugin.schema.json','name':'a'}
 (p/'plugin.json').write_text(json.dumps(manifest))
 marker=t/'EXECUTED';sentinel=p/'sentinel';sentinel.write_text('#!/bin/sh\nprintf x > '+shlex.quote(str(marker))+'\n');sentinel.chmod(0o755)
 (p/'mcp.json').write_text(json.dumps({'$schema':S+'mcp.schema.json','mcpServers':{'local':{'type':'stdio','command':'./sentinel'},'remote':{'type':'sse','url':'https://127.0.0.1:9/mcp'}}}))
 def snapshot(path):
  result={}
  for root,dirs,files in os.walk(path,followlinks=False):
   for name in dirs+files:
    f=P.Path(root)/name;k=str(f.relative_to(path));st=f.lstat()
    result[k]=(st.st_mode,os.readlink(f) if f.is_symlink() else hashlib.sha256(f.read_bytes()).hexdigest() if f.is_file() else None)
  return result
 def traced(label,path,expected):
  before=snapshot(path)
  log=t/(label+'.trace');q=subprocess.run(['strace','-f','-e','trace=network','-o',str(log),str(binary),str(path),'--json'],cwd=t,capture_output=True,text=True,timeout=30)
  r=json.loads(q.stdout);assert q.returncode==expected and r['exitCode']==expected,(label,q.returncode,r)
  lines=log.read_text().splitlines();calls=[line for line in lines if '(' in line];assert not calls,(label,calls);assert not marker.exists(),'校验器执行了被检命令'
  assert snapshot(path)==before,'校验器修改了被检输入'
  return {'case':label,'exitCode':q.returncode,'networkSyscalls':len(calls),'plugins':len(r['plugins']),'inputUnchanged':True}
 results=[traced('standalone-mcp',p,0)]
 manifest['$schema']='https://schema-fetch-must-not-happen.invalid/plugin.schema.json';(p/'plugin.json').write_text(json.dumps(manifest));results.append(traced('unknown-schema',p,1))
 corpus=os.environ.get('AP_LINT_CORPUS')
 if corpus:results.append(traced('corpus',P.Path(corpus).resolve(),1))
 print(json.dumps({'verified':results,'commandExecuted':False,'limit':'仅观察这些输入的系统调用；不是网络命名空间隔离或形式化证明。'},ensure_ascii=False,indent=2))
