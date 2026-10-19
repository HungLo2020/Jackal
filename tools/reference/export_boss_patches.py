#!/usr/bin/env python3
"""Export original F33D/DF63/DF67/DF6E background patches as native tile JSON."""
import argparse
import json
import subprocess
from export_graphics import ROOT, REFERENCE
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
data=subprocess.check_output(['git','show',f'{REFERENCE}:assets/game-data.bin'],cwd=ROOT)
def byte(address): return data[7*16384+address-0xc000]
def signed(value): return value if value<128 else value-256
patches=[]
for index in range(29):
    pointer=byte(0xe099+index*2)+256*byte(0xe09a+index*2)
    mode=1 if 8<=index<12 else 2 if index in (14,26) else 0
    size=byte(0xdfd6+mode)
    patches.append(dict(id=index,width=size,height=size,offset_x=signed(byte(0xf394+index)),offset_y=signed(byte(0xf3b1+index)),collision=byte(pointer+size*size),tiles=[byte(pointer+i) for i in range(size*size)]))
result=json.dumps(dict(patches=patches),indent=2)+'\n'
output=ROOT/'assets/boss_patches.json'
if args.check:
    if result!=output.read_text():raise SystemExit('Boss tile patches differ from reference')
    print('Boss tile patches verified')
else:output.write_text(result)
