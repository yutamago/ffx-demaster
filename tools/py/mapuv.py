import sys,collections,os; sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import numpy as np, hdworld, mapcmp, glbstat, compare2

def prims_world(path):
    js,bb=glbstat.load(path)
    out=[]
    def walk(i,M):
        n=js['nodes'][i]; W=M@hdworld.local(n)
        if 'mesh' in n:
            for p in js['meshes'][n['mesh']]['primitives']:
                if 'TEXCOORD_0' not in p['attributes']: continue
                P=np.array(glbstat.accessor(js,bb,p['attributes']['POSITION'])[0])
                U=np.array(glbstat.accessor(js,bb,p['attributes']['TEXCOORD_0'])[0])
                out.append(((W[:3,:3]@P.T).T+W[:3,3], U))
        for c in n.get('children',[]): walk(c,W)
    for r in js['scenes'][js.get('scene',0)]['nodes']: walk(r,np.eye(4))
    return out

def run(vpa, glb):
    import ps2mapgeom
    bt,_=ps2mapgeom.batches(vpa)
    P=np.concatenate([p for p,u in bt]); UV=np.concatenate([u for p,u in bt])
    d=collections.defaultdict(list)
    for k,uv in zip(map(tuple,np.round(P*20).astype(int)),UV): d[k].append(uv)
    tot=fit_ok=matched=0
    for H,HU in prims_world(glb):
        H=H*np.array([-1,-1,1])
        pairs=[];idx=[]
        for i,(k,uv) in enumerate(zip(map(tuple,np.round(H*20).astype(int)),HU)):
            tot+=1
            if k in d:
                for c in d[k]: pairs.append((c[0],c[1],uv[0],uv[1])); idx.append(i)
        if not pairs: continue
        a=np.array(pairs); mask,prm=compare2.fit(a)
        ok=set(i for i,f in zip(idx,mask) if f)
        matched+=len(set(idx)); fit_ok+=len(ok)
    return {'map':os.path.basename(glb),'hd_verts':tot,'pos_matched':round(matched/tot,3),'uv_affine_fit_of_matched':round(fit_ok/max(1,matched),3)}

if __name__=='__main__':
    a=sys.argv[1:]
    for v,g in zip(a[::2],a[1::2]): print(run(v,g),flush=True)
