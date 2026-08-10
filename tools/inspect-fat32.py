#!/usr/bin/env python3
from __future__ import annotations
import argparse, struct
from pathlib import Path

SECTOR=512

def short(component: str) -> bytes:
    stem, _, ext = component.upper().partition('.')
    return stem.encode().ljust(8,b' ') + ext.encode().ljust(3,b' ')

def main() -> int:
    ap=argparse.ArgumentParser(); ap.add_argument('image',type=Path); args=ap.parse_args()
    data=args.image.read_bytes()
    assert len(data)%SECTOR==0 and data[510:512]==b'\x55\xaa'
    bps=struct.unpack_from('<H',data,11)[0]; spc=data[13]; reserved=struct.unpack_from('<H',data,14)[0]
    fats=data[16]; fatsz=struct.unpack_from('<I',data,36)[0]; root=struct.unpack_from('<I',data,44)[0]
    assert bps==SECTOR and spc==1 and root==2
    first_data=reserved+fats*fatsz
    def coff(cluster): return (first_data+(cluster-2)*spc)*SECTOR
    def entries(cluster):
        block=data[coff(cluster):coff(cluster)+SECTOR]
        out=[]
        for off in range(0,SECTOR,32):
            if block[off]==0: break
            if block[off] in (0xe5,): continue
            attr=block[off+11]
            if attr in (0x0f,) or attr&0x08: continue
            name=bytes(block[off:off+11]); hi=struct.unpack_from('<H',block,off+20)[0]; lo=struct.unpack_from('<H',block,off+26)[0]
            size=struct.unpack_from('<I',block,off+28)[0]
            out.append((name,attr,(hi<<16)|lo,size))
        return out
    root_e=entries(root); system=next(e for e in root_e if e[0]==short('SYSTEM'))
    sys_e=entries(system[2]); services=next(e for e in sys_e if e[0]==short('SERVICES'))
    svc_e=entries(services[2]); names={e[0] for e in svc_e}
    for required in ['INIT.ELF','LOGD.ELF','CONSOL.ELF','DISPLAYD.ELF','ARCHIVE.ELF','TRUSTD.ELF','DRIVERD.ELF','AUDIOD.ELF','SHELL.ELF','SERVICES.CFG']:
        assert short(required) in names, required
    print(f'FAT32 image valid: {len(data)} bytes, {len(svc_e)} service entries')
    return 0
if __name__=='__main__': raise SystemExit(main())
