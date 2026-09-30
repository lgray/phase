# Usage (from a repo root): python3 count_reads_lines.py [REV [PATTERN]] [-v]   REV defaults to HEAD; REPO=<path> overrides the cwd.
import subprocess,re,os,collections,sys
ARGS=[a for a in sys.argv[1:] if a!='-v']
REV=ARGS[0] if ARGS else 'HEAD'; R=os.environ.get('REPO','.')
PATSTR=ARGS[1] if len(ARGS)>1 else r'\.(library|graveyard)\b(?!\s*\()'
def git(*a): return subprocess.run(['git','-C',R,*a],capture_output=True,text=True).stdout
roots=['crates/engine/src','crates/phase-ai/src','crates/engine-wasm/src','crates/phase-llm/src']
files=[f for r in roots for f in git('ls-tree','-r','--name-only',REV,'--',r).split() if f.endswith('.rs')]
src={f:git('show',f'{REV}:{f}') for f in files}
TESTATTR=re.compile(r'#\[cfg\((test|any\(test\b[^\]]*)\)\]')
testmods=set(); prodmods=set()
for f,s in src.items():
    L=s.split('\n'); d=os.path.dirname(f); b=os.path.basename(f)
    base=d if b in('mod.rs','lib.rs','main.rs') else os.path.join(d,b[:-3])
    for i,l in enumerate(L):
        if not TESTATTR.search(l):
            m=re.match(r'\s*(pub(\([a-z]+\))?\s+)?mod (\w+);',l)
            if m and not (i>0 and re.search(r'#\[cfg\(', L[i-1])):
                prodmods.add(os.path.join(base,m.group(3)+'.rs'))
            elif m and i>0 and 'cfg(not(' in L[i-1]: prodmods.add(os.path.join(base,m.group(3)+'.rs')); prodmods.add(os.path.join(base,m.group(3))+'/')
            continue
        path=None
        for j in range(i+1,min(i+6,len(L))):
            m=re.match(r'\s*#\[path\s*=\s*"([^"]+)"\]',L[j])
            if m: path=m.group(1); continue
            if L[j].strip().startswith('#['): continue
            m=re.match(r'\s*(pub(\([a-z]+\))?\s+)?mod (\w+);',L[j])
            if m:
                if path: testmods.add(os.path.normpath(os.path.join(d,path)))
                else: testmods.add(os.path.join(base,m.group(3)+'.rs')); testmods.add(os.path.join(base,m.group(3))+'/')
            break
def is_test_file(f):
    if re.search(r"(_tests?\.rs$|/tests?/|/tests?\.rs$|/test_[a-z_]+\.rs$)", f): return True
    if f in prodmods: return False
    return f in testmods or any(t.endswith('/') and f.startswith(t) and not any(f.startswith(p) for p in prodmods if p.endswith('/')) for t in testmods) or '/bin/' in f
PAT=re.compile(PATSTR)
LINES=[]; counts=collections.Counter(); perfile=collections.Counter(); excluded=0
for f,s in src.items():
    crate=f.split('/')[1]
    if is_test_file(f): excluded+=len(PAT.findall(s)); continue
    depth=0; skip=False; pending=False; skipdepth=0
    for l in s.split('\n'):
        code=re.sub(r'//.*','',re.sub(r'"(\\.|[^"\\])*"','""',l)).replace("'{'","").replace("'}'","")
        if not skip and TESTATTR.search(l): pending=True; continue
        if pending and not skip:
            if code.strip().startswith('#['): continue
            if '{' in code: skip=True; skipdepth=depth
            elif code.strip().endswith(';'): pending=False; continue
        o=code.count('{'); c=code.count('}')
        depth+=o-c
        if skip:
            if depth<=skipdepth: skip=False; pending=False
            continue
        n=len(PAT.findall(code))
        if n: counts[crate]+=n; perfile[f]+=n; LINES.append((f,l.strip()))
print('occurrences by crate:',dict(counts)); print('files by crate:',dict(collections.Counter(f.split('/')[1] for f in perfile)))
print('excluded (test-only files):',excluded)
if '-v' in sys.argv:
    for f,n in perfile.most_common(): print(n,f)
for f,l in LINES: print(f.replace("crates/",""),"|",l[:110])
