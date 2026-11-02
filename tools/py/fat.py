import struct, sys
def load_mdg(path):
    d=open(path,'rb').read(); return list(struct.unpack('<%dI'%(len(d)//4), d))
def sizes(ent, sizetbl=None):
    out=[]
    for i,e in enumerate(ent):
        st=e&0x3fffff
        if e&0x800000 or i+1>=len(ent): out.append((st,None)); continue
        if e&0x400000:
            if sizetbl is None: raise ValueError('need sizetbl')
            b=sizetbl[3*i:3*i+3]; out.append((st, int.from_bytes(b,'little')<<3)); continue
        nx=ent[i+1]&0x3fffff
        out.append((st, ((nx-st)<<11) - ((e>>24)<<3)))
    return out
def load_fnd(path):
    d=open(path,'rb').read(); n=struct.unpack_from('<I',d,0)[0]
    offs=struct.unpack_from('<%dI'%(n+1),d,4)
    names=[]
    for i in range(n):
        s,e=offs[i],offs[i+1]
        names.append(d[s:e].split(b'\0')[0].decode('latin1') if e>s else None)
    return names
