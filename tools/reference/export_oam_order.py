#!/usr/bin/env python3
"""Capture isolated original boss actor sequences; normal builds never run this."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from export_graphics import ROOT,HERE,REFERENCE,FILES
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
with tempfile.TemporaryDirectory(prefix='jackal-oam-reference-') as temp:
    root=Path(temp)
    for name in FILES:
        target=root/name;target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(subprocess.check_output(['git','show',f'{REFERENCE}:{name}'],cwd=ROOT))
    manifest=root/'Cargo.toml'
    manifest.write_text(manifest.read_text().replace('rodio = "0.20.1"','rodio = "0.20.1"\nserde_json = "1.0"'))
    with (root/'src/game.rs').open('a') as stream:stream.write((HERE/'enemy_sequences_methods.rs').read_text())
    with (root/'src/game.rs').open('a') as stream:stream.write('\nimpl Game {pub fn draw_call(&mut self) -> Result<(),String> {self.bank=1;self.memory[0x27]=1;self.push_word(0xc28e);self.block=0x8008;self.run_until_idle()}}\n')
    (root/'src/main.rs').write_text((HERE/'oam_order_main.rs').read_text())
    raw=subprocess.check_output(['cargo','run','--offline','--manifest-path',str(manifest)],cwd=root,env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target/reference-oam')))
    result=json.dumps(json.loads(raw),separators=(',',':'))+'\n'
    output=ROOT/'tests/fixtures/oam_order.json'
    if args.check:
        if result!=output.read_text():raise SystemExit('OAM ordering reference differs')
        print('OAM ordering reference verified')
    else:output.write_text(result)
