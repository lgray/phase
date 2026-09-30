# Usage (from a repo root): python3 matches.py [REV]   REV defaults to HEAD; REPO=<path> overrides the cwd.
import os, re, subprocess, sys
REF=sys.argv[1] if len(sys.argv)>1 else 'HEAD'; R=os.environ.get('REPO','.')
files=subprocess.run(['git','-C',R,'grep','-l','-P','GameFormat::',REF,'--','*.rs'],capture_output=True,text=True).stdout.split()
files=[f.split(':',1)[1] for f in files]
def strip(src):
    # blank out comments and string/char literals, preserving length/newlines
    out=list(src); i=0; n=len(src)
    while i<n:
        c=src[i]
        if src.startswith('//',i):
            j=src.find('\n',i); j=n if j<0 else j
            for k in range(i,j): out[k]=' '
            i=j
        elif src.startswith('/*',i):
            j=src.find('*/',i)+2
            for k in range(i,j):
                if src[k]!='\n': out[k]=' '
            i=j
        elif c=='"' :
            j=i+1
            while j<n and src[j]!='"':
                j+= 2 if src[j]=='\\' else 1
            for k in range(i+1,j):
                if src[k]!='\n': out[k]=' '
            i=j+1
        elif c=="'" and i+2<n and (src[i+2]=="'" or (src[i+1]=='\\')):
            j=src.find("'",i+2 if src[i+1]=='\\' else i+1)
            for k in range(i+1,j): out[k]=' '
            i=j+1
        else: i+=1
    return ''.join(out)
res=[]
for f in files:
    src=subprocess.run(['git','-C',R,'show',f'{REF}:{f}'],capture_output=True,text=True).stdout
    s=strip(src)
    testspans=[]
    for tm in re.finditer(r'#\[cfg\(test\)\]\s*(pub(\([a-z]+\))?\s+)?mod\s+\w+\s*\{',s):
        j=tm.end(); d=1
        while d:
            d+={'{':1,'}':-1}.get(s[j],0); j+=1
        testspans.append((tm.start(),j))
    for m in re.finditer(r'\bmatch\b',s):
        # find opening brace of match body at depth 0 relative
        i=m.end(); depth=0
        while i<len(s):
            if s[i] in '([': depth+=1
            elif s[i] in ')]': depth-=1
            elif s[i]=='{' and depth==0: break
            elif s[i]==';': i=-1; break
            i+=1
        if i<0 or i>=len(s): continue
        scrut=s[m.end():i].strip()
        # body
        j=i+1; d=1
        while d: 
            if s[j]=='{': d+=1
            elif s[j]=='}': d-=1
            j+=1
        body=s[i+1:j-1]
        # top-level arm patterns: text at depth0 between arm separators ending with =>
        pats=[]; d=0; start=0; k=0
        while k<len(body):
            ch=body[k]
            if ch in '([{': d+=1
            elif ch in ')]}': 
                d-=1
                if d==0 and ch=='}':
                    # block-bodied arm end; next arm starts after optional comma
                    start=k+1
            elif ch==',' and d==0: start=k+1
            elif body.startswith('=>',k) and d==0:
                pats.append(body[start:k].strip()); start=k+2; k+=1
            k+=1
        if not any(re.match(r'^\|?\s*GameFormat::',p) for p in pats): continue
        catch=[p for p in pats if re.fullmatch(r'_( if .*)?',p,re.S) or re.fullmatch(r'[a-z_][a-z0-9_]*( if .*)?',p,re.S)]
        line=s.count('\n',0,m.start())+1
        fn=None
        for fm in re.finditer(r'\bfn\s+(\w+)',s[:m.start()]): fn=fm.group(1)
        res.append((f,line,fn,scrut,'WILDCARD:'+'|'.join(c.split()[0] for c in catch) if catch else 'EXHAUSTIVE','test' if any(a<=m.start()<b for a,b in testspans) or '/tests/' in f else 'prod'))
for r in res: print('\t'.join(map(str,r)))
