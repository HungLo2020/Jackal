"""Ahead-of-time lowering for the single SHA-256 pinned Jackal USA ROM.
Every legal instruction offset is a candidate entry; this overapproximates code
coverage and does NOT establish which bytes are actually code. No decoder is
shipped in the executable. Original bank/address pairs remain source provenance.
Lowering patterns adapted from the local MicroMages reconstruction compiler.
"""
import hashlib,json,sys
from pathlib import Path
image=Path(sys.argv[1]).read_bytes()
expected="97e0e7d84ce6cca3fd3ec5eabb3fa00252de54f1354156ca2b95b95a39237373"
assert hashlib.sha256(image).hexdigest()==expected,"Unrecognized Jackal ROM"
out=Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
prg=image[16:]
ops={int(k):v for k,v in json.loads(Path("tools/instructions.json").read_text()).items()}
lengths=dict(imp=1,acc=1,imm=2,zpg=2,zpx=2,zpy=2,inx=2,iny=2,rel=2,abs=3,abx=3,aby=3,ind=3)
branches=dict(BPL="!s.n",BMI="s.n",BVC="!s.v",BVS="s.v",BCC="!s.c",BCS="s.c",BNE="!s.z",BEQ="s.z")
def address(mode, arg):
    a = f"0x{arg:04x}u16"
    return {
        "zpg": a, "abs": a,
        "zpx": f"(0x{arg:02x}u8.wrapping_add(s.x)) as u16",
        "zpy": f"(0x{arg:02x}u8.wrapping_add(s.y)) as u16",
        "abx": f"{a}.wrapping_add(s.x as u16)",
        "aby": f"{a}.wrapping_add(s.y as u16)",
        "inx": f"s.pointer(0x{arg:02x}u8.wrapping_add(s.x))",
        "iny": f"s.pointer(0x{arg:02x}).wrapping_add(s.y as u16)",
    }[mode]

def emit(addr, name, mode, arg, nxt):
    lines = [f"// ${addr:04x}: {name} {mode} ${arg:04x}"]
    if name in branches:
        target = (nxt + (arg if arg < 128 else arg - 256)) & 0xffff
        lines.append(f"return Ok(if {branches[name]} {{ 0x{target:04x} }} else {{ 0x{nxt:04x} }});")
    elif name == "JSR":
        lines += [f"s.push_word(0x{nxt - 1:04x});", f"", f"return Ok(0x{arg:04x});"]
    elif name == "JMP":
        if mode == "abs":
            lines += [f"", f"return Ok(0x{arg:04x});"]
        else:
            lines += [f"let lo = s.read(0x{arg:04x}) as u16;", f"let hi = s.read(0x{(arg & 0xff00) | ((arg + 1) & 0xff):04x}) as u16;", "let target = lo | hi << 8; return Ok(target);"]
    elif name == "RTS":
        lines += ["return Ok(s.pop_word().wrapping_add(1));"]
    elif name == "RTI":
        lines += ["let flags = s.pop(); s.set_flags(flags); return Ok(s.pop_word());"]
    elif name == "FRAME":
        lines += [f"s.frame_ready = true; return Ok(0x{nxt:04x});"]
    elif name == "BRK":
        # The game uses BRK as an animation-pointer call. Its IRQ routine consumes
        # the saved return location and the two inline data bytes, then RTSes.
        lines += [f"s.push_word(0x{(addr+2)&0xffff:04x});", "s.push(s.flags() | 0x30); s.interrupt_mask = true; return Ok(0xc2ea);"]
    elif name in ("LDA", "LDX", "LDY", "ORA", "AND", "EOR", "ADC", "SBC", "CMP", "CPX", "CPY", "BIT"):
        if mode == "imm":
            lines.append(f"let value = 0x{arg:02x}u8;")
        else:
            lines += [f"let addr = {address(mode, arg)};", "let value = s.read(addr);"]
        if name.startswith("LD"):
            r = name[-1].lower()
            lines += [f"s.{r} = value; s.nz(value);"]
        elif name in ("ORA", "AND", "EOR"):
            op = dict(ORA="|", AND="&", EOR="^")[name]
            lines += [f"s.a {op}= value; s.nz(s.a);"]
        elif name == "ADC": lines += ["s.add(value);"]
        elif name == "SBC": lines += ["s.add(!value);"]
        elif name in ("CMP", "CPX", "CPY"):
            r = dict(CMP="a", CPX="x", CPY="y")[name]
            lines += [f"s.c = s.{r} >= value; s.nz(s.{r}.wrapping_sub(value));"]
        else: lines += ["s.z = s.a & value == 0; s.n = value & 0x80 != 0; s.v = value & 0x40 != 0;"]
    elif name in ("STA", "STX", "STY"):
        lines += [f"let addr = {address(mode,arg)};", f"s.write(addr, s.{name[-1].lower()});"]
    elif name in ("INC", "DEC", "ASL", "LSR", "ROL", "ROR"):
        if mode == "acc": lines += ["let value = s.a;"]
        else: lines += [f"let addr = {address(mode,arg)};", "let value = s.read(addr);"]
        operation = {
            "INC": "let value = value.wrapping_add(1);",
            "DEC": "let value = value.wrapping_sub(1);",
            "ASL": "s.c = value & 128 != 0; let value = value << 1;",
            "LSR": "s.c = value & 1 != 0; let value = value >> 1;",
            "ROL": "let carry = s.c as u8; s.c = value & 128 != 0; let value = (value << 1) | carry;",
            "ROR": "let carry = (s.c as u8) << 7; s.c = value & 1 != 0; let value = (value >> 1) | carry;",
        }[name]
        lines += [operation, "s.nz(value);", "s.a = value;" if mode == "acc" else "s.write(addr, value);"]
    elif name in ("TAX", "TAY", "TXA", "TYA", "TSX", "TXS"):
        fr, to = name[1].lower(), name[2].lower()
        fr = "sp" if fr == "s" else fr
        to = "sp" if to == "s" else to
        lines += [f"s.{to} = s.{fr};"]
        if to != "sp": lines += [f"s.nz(s.{to});"]
    elif name in ("INX", "INY", "DEX", "DEY"):
        r = name[-1].lower(); op = "add" if name.startswith("IN") else "sub"
        lines += [f"s.{r} = s.{r}.wrapping_{op}(1); s.nz(s.{r});"]
    elif name == "PHA": lines += ["s.push(s.a);"]
    elif name == "PLA": lines += ["s.a = s.pop(); s.nz(s.a);"]
    elif name == "PHP": lines += ["s.push(s.flags() | 0x30);"]
    elif name == "PLP": lines += ["let flags = s.pop(); s.set_flags(flags);"]
    elif name in ("CLC", "SEC"): lines += [f"s.c = {'true' if name == 'SEC' else 'false'};"]
    elif name == "CLV": lines += ["s.v = false;"]
    elif name in ("CLI", "SEI"): lines += [f"s.interrupt_mask = {'true' if name == 'SEI' else 'false'};"]
    elif name in ("CLD", "SED"): lines += [f"s.decimal = {'true' if name == 'SED' else 'false'};"]
    elif name == "NOP": pass
    else: raise ValueError((addr, name, mode))
    # Per-instruction temporary scope prevents accidental reuse of lowered locals.
    return ["{", *lines, "}"]


def timing(name,mode,arg,addr,nxt):
    # Source timing annotations only position the story's raster split. Native
    # frame scheduling itself calls the recovered NMI routine directly.
    if name in branches:
        target=(nxt+(arg if arg<128 else arg-256))&65535
        taken=1+int((nxt&0xff00)!=(target&0xff00))
        return f"s.elapsed += 2 + if {branches[name]} {{ {taken} }} else {{ 0 }};"
    fixed=dict(BRK=7,JSR=6,RTS=6,RTI=6,PHA=3,PHP=3,PLA=4,PLP=4)
    if name in fixed:return f"s.elapsed += {fixed[name]};"
    if name=='JMP':return f"s.elapsed += {5 if mode=='ind' else 3};"
    rmw=name in ('INC','DEC','ASL','LSR','ROL','ROR') and mode!='acc'
    store=name in ('STA','STX','STY')
    cost=dict(imp=2,acc=2,imm=2,zpg=3,zpx=4,zpy=4,abs=4,abx=4,aby=4,inx=6,iny=5)[mode]
    if rmw:cost+=2+int(mode in ('abx','aby'))
    elif store and mode in ('abx','aby','iny'):cost+=1
    result=f"s.elapsed += {cost};"
    if not rmw and not store and mode in ('abx','aby','iny'):
        base=f'0x{arg:04x}u16' if mode!='iny' else f's.pointer(0x{arg:02x})'
        index='s.x' if mode=='abx' else 's.y'
        result+=f"s.elapsed += ((({base} & 255) + {index} as u16) > 255) as u32;"
    return result

count=0
main=["// Generated native code; no runtime instruction decoding.","pub fn dispatch(s: &mut super::Game, address: u16) -> Result<u16,String> {", "let bank = if address >= 0xc000 { 7 } else { s.bank };", "match (bank, address >> 8) {"]
for bank in range(8):
 base=0xc000 if bank==7 else 0x8000
 for page in range(base>>8,(base+16384)>>8):
  name=f"b{bank}_p{page:02x}"
  main.append(f"({bank}, {page}) => {name}(s,address),")
main+=['_ => Err(format!("Transfer outside translated ROM: bank {} ${:04x}",s.bank,address)),',"}","}"]
for bank in range(8):
 base=0xc000 if bank==7 else 0x8000
 for page in range(base>>8,(base+16384)>>8):
  name=f"b{bank}_p{page:02x}"
  main += ['#[allow(unreachable_code, unused_variables)]',f"fn {name}(s:&mut super::Game,address:u16)->Result<u16,String>{{match address {{"]
  for addr in range(page*256,(page+1)*256):
   offset=bank*16384+addr-base
   opcode=prg[offset]
   if opcode not in ops:continue
   mnemonic,mode=ops[opcode];size=lengths[mode]
   # At the switchable-window boundary operands come from the fixed bank.
   def byte(delta):
    location=addr+delta
    if location>65535:return 0
    if bank!=7 and location>=0xc000:return prg[7*16384+location-0xc000]
    return prg[offset+delta]
   arg=byte(1)|(byte(2)<<8) if size==3 else byte(1) if size==2 else 0
   nxt=(addr+size)&65535
   lines=[timing(mnemonic,mode,arg,addr,nxt),*emit(addr,mnemonic,mode,arg,nxt)]
   if mnemonic not in branches and mnemonic not in ('JMP','JSR','RTS','RTI','BRK'):
    lines.append(f"Ok(0x{nxt:04x})")
   main += [f"0x{addr:04x}=>{{",*lines,"},"]
   count+=1
  main += ['_ => Err(format!("Unsupported translated entry bank {} ${:04x}",s.bank,address)),','}}']
(out/'native_game.rs').write_text('\n'.join(main)+'\n')
(out/'prg.bin').write_bytes(prg)
(out/'translation.json').write_text(json.dumps(dict(rom_sha256=expected,candidate_entries=count,coverage="All official-opcode offsets; not all entries are code"),indent=2)+'\n')
print(f"Lowered {count} candidate entries",file=sys.stderr)
