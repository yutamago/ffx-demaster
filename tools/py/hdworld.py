import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np, glbstat

def qmat(q):
    x,y,z,w=q
    return np.array([[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],[2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],[2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]])

def local(n):
    if 'matrix' in n: return np.array(n['matrix']).reshape(4,4).T
    M=np.eye(4)
    R=qmat(n['rotation']) if 'rotation' in n else np.eye(3)
    S=np.diag(n.get('scale',[1,1,1]))
    M[:3,:3]=R@S; M[:3,3]=n.get('translation',[0,0,0])
    return M

def world_positions(path):
    js,bb=glbstat.load(path)
    out=[]
    def walk(i,M):
        n=js['nodes'][i]; W=M@local(n)
        if 'mesh' in n:
            for p in js['meshes'][n['mesh']]['primitives']:
                P=np.array(glbstat.accessor(js,bb,p['attributes']['POSITION'])[0])
                out.append((W[:3,:3]@P.T).T+W[:3,3])
        for c in n.get('children',[]): walk(c,W)
    for r in js['scenes'][js.get('scene',0)]['nodes']: walk(r,np.eye(4))
    return np.concatenate(out) if out else np.zeros((0,3))

def world_positions_uv(path):
    js,bb=glbstat.load(path)
    P_,U_=[],[]
    def walk(i,M):
        n=js['nodes'][i]; W=M@local(n)
        if 'mesh' in n:
            for p in js['meshes'][n['mesh']]['primitives']:
                P=np.array(glbstat.accessor(js,bb,p['attributes']['POSITION'])[0])
                if 'TEXCOORD_0' not in p['attributes']: continue
                U=np.array(glbstat.accessor(js,bb,p['attributes']['TEXCOORD_0'])[0])
                P_.append((W[:3,:3]@P.T).T+W[:3,3]); U_.append(U)
        for c in n.get('children',[]): walk(c,W)
    for r in js['scenes'][js.get('scene',0)]['nodes']: walk(r,np.eye(4))
    return np.concatenate(P_), np.concatenate(U_)
