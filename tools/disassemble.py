import json,sys
from pathlib import Path
OPS=json.loads(Path(__file__).with_name('instructions.json').read_text())
SIZES={'imp':1,'acc':1,'imm':2,'zpg':2,'zpx':2,'zpy':2,'inx':2,'iny':2,'rel':2,'abs':3,'abx':3,'aby':3,'ind':3}
rom=Path('assets/jackal.nes').read_bytes()[16:]
bank=int(sys.argv[1],0) if len(sys.argv)>1 else 7
base=0xc000 if bank==7 else 0x8000
start=int(sys.argv[2],16) if len(sys.argv)>2 else base
end=int(sys.argv[3],16) if len(sys.argv)>3 else base+0x4000
data=rom[bank*0x4000:(bank+1)*0x4000]
pc=start
while pc<end:
 b=data[pc-base]; op,mode=OPS.get(str(b),('???','imp')); n=SIZES[mode]; bs=data[pc-base:pc-base+n]; v=int.from_bytes(bs[1:],'little')
 arg='' if n==1 else f'${v:04x}' if n==3 else f'${v:02x}'
 if mode=='rel':arg=f'${(pc+2+(v if v<128 else v-256))&65535:04x}'
 print(f'{pc:04x} {bs.hex(" "):8s} {op} {arg} ({mode})')
 pc+=n
