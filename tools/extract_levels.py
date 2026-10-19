#!/usr/bin/env python3
"""Static data extraction from the supplied Jackal PRG. No CPU interpretation."""
import argparse,json
from pathlib import Path
BANKS=[4,4,5,4,4,5]
BOSSES=['fortress_guns','battleship_guns','statue_battery','attack_aircraft','heavy_tank','enemy_headquarters']
NAMES=['Coastal Assault','River Crossing','Ancient Ruins','Mountain Fortress','Industrial Base','Enemy Headquarters']
# Categories whose routine/data identity has been established; other identities remain explicit.
KINDS={1:'infantry',2:'infantry',3:'infantry',9:'boss_fortress_guns',0x20:'boss_battleship_guns',0x25:'boss_statue_battery',0x40:'boss_attack_aircraft',0x17:'boss_heavy_tank',0x49:'headquarters_gate',0x45:'headquarters_core',0x4b:'headquarters_building',0x16:'prisoner',0x27:'rescued_prisoner',0x3f:'rescue_zone',0x44:'terrain_trigger',0x50:'bomb_pickup',0x51:'weapon_pickup',0x52:'extra_life_pickup'}
KINDS.update({4:'infantry_patrol_variant',5:'rotating_turret',6:'rotating_turret_variant',7:'armored_vehicle',8:'stationary_cannon',0x0a:'boss_attack_tank',0x13:'prisoner_hut',0x14:'prisoner_hut_variant',0x15:'prisoner_hut_small',0x19:'armored_vehicle_garage',0x1c:'prisoner_hut_large',0x1d:'prisoner_hut_large_variant',0x3c:'helicopter_flyover',0x3d:'rescue_helicopter_left',0x3e:'rescue_helicopter_right',0x18:'boss_gun_tower',0x26:'boss_shielded_core',0x31:'boss_guard_turret',0x4a:'headquarters_turret'})
# Families with uncertain visual identity are identified as provisional in analysis output.
FAMILIES={
 'infantry':[1,2,3,4], 'turret':[0xc,0x10,0x1f,0x21,0x22,0x2a,0x2b,0x2c,0x2d,0x2e,0x2f,0x30,0x4e,0x53],
 'vehicle':[5,6,7,0xb,0xe,0xf,0x11,0x12,0x29,0x43], 'boat':[8,0x1a],
 'prison':[0x13,0x14,0x15,0x19,0x1c,0x1d], 'destructible':[0x1b,0x3a,0x3b],
 'environment':[0x32,0x35,0x38,0x44,0x49], 'helicopter':[0x3c,0x3d,0x3e],
 'rescue_zone':[0x3f], 'pickup':[0x50,0x51,0x52], 'prisoner':[0x16,0x27,0x28],
 'boss':[9,0xa,0x17,0x18,0x20,0x25,0x26,0x31,0x40,0x45,0x4a,0x4b]}
KINDS.update({5:'tank_rotating_gun',6:'tank_rotating_gun_heavy',7:'armored_vehicle',8:'patrol_boat',0xb:'tank_medium',0xe:'tank_medium_variant',0xf:'tank_light',0x11:'military_truck',0x1a:'gunboat',0x2a:'missile_launcher_left',0x2b:'missile_launcher_right',0x2c:'fixed_launcher',0x2d:'retracting_launcher',0x2e:'heavy_fixed_launcher',0x2f:'aimed_cannon_left',0x30:'aimed_cannon_right'})
FAMILY_BY_ID={i:f for f,ids in FAMILIES.items() for i in ids}
for _family,_ids in FAMILIES.items():
 for _id in _ids:KINDS.setdefault(_id,f'{_family}_variant_{_id:02x}')
# Explicit known initialization frames (stage-specific frames can override these).
FRAMES={1:0x12,2:0x12,3:0x12,4:0x12,5:0x1c,6:0x1c,7:0x22,8:0x3a,0xa:0x33,
 0xb:0x44,0xc:0x81,0xe:0x44,0xf:0x3d,0x11:0x54,0x12:0x50,0x16:0x25,
 0x1a:0x56,0x29:0x5e,0x2a:0x67,0x2b:0x67,0x2c:0x69,0x2d:0x6c,0x2e:0x9f,
 0x2f:0x7c,0x30:0x7c,0x3c:0x31,0x3d:0x2f,0x3e:0x2f,0x43:0x9b,0x50:0x2e,0x51:0x2e,0x52:0x2e}

def extract(prg):
 def data(b,a,n=1):return prg[b*16384+a-0x8000:b*16384+a-0x8000+n] if a<0xc000 else prg[7*16384+a-0xc000:7*16384+a-0xc000+n]
 def byte(b,a):return data(b,a)[0]
 def word(b,a):return int.from_bytes(data(b,a,2),'little')
 stages=[]
 for s,b in enumerate(BANKS):
  sequence=word(7,0xcff4+s*2);pages=word(7,0xd000+s*2);defs=word(7,0xd00c+s*2);attrs=word(7,0xd018+s*2)
  order=list(data(b,sequence,pages-sequence));cutoff=byte(7,0xcfee+s)
  # Preserve the final repeated page: the runtime streams it into the boss viewport.
  rows=[];prows=[];crows=[];thresholds=list(data(7,0xcfbe+s*8,6))
  def collision(t):
   a,b,c,d,e,f=thresholds
   return 0 if t<a else 1 if t<b else 2 if t<c else 0 if t<d else 1 if t<e else 2 if t<f else 3
  for page in order:
   for my in range(8):
    mids=list(data(b,pages+page*128+my*16,16))
    for sy in range(4):
     # CD88-CD90 restarts every source page at subrow 2: 30 streamed rows.
     if my == 0 and sy < 2:continue
     row=[];prow=[]
     for mid in mids:
      db=4 if mid<cutoff else b
      base=word(7,0xd00c) if mid<cutoff else defs
      idx=mid if mid<cutoff else mid-cutoff
      ts=list(data(db,base+idx*16+sy*4,4));row+=ts
      attr=byte(b,attrs+mid)
      prow += [(attr>>((1-sy//2)*4+(sx//2)*2))&3 for sx in range(4)]
     rows.append(row);prows.append(prow);crows.append([collision(t) for t in row])
  rows.reverse();prows.reverse();crows.reverse()
  events=[];commands=[];event_table=word(7,0xf1e0+s*2)
  for page in range(12 if s<5 else 14):
   a=word(6,event_table+page*2);offset=0
   while offset<256:
    trigger=byte(6,a+offset)
    if trigger==0xef:break
    if trigger in [0xf0,0xf1,0xf2]:
     commands.append({'distance':page*240+byte(6,a+offset+1),'command':{0xf0:'forced_scroll',0xf1:'palette_prepare',0xf2:'palette_change'}[trigger]});offset+=2;continue
    x,t=data(6,a+offset+1,2);kind=KINDS.get(t&127,f'entity_{t&127:02x}')
    events.append({'distance':float(page*240+trigger),'x':float((x&127)*4),'kind':kind,'source_type':t&127,'priority_spawn':bool(t&128),'raw_source_type':t,'from_bottom':bool(x&128),'odd_difficulty_only':bool(trigger&1),'source_page':page})
    offset+=3
  stages.append({'name':NAMES[s],'width_tiles':64,'height_tiles':len(rows),'tile_size':8,'tiles':sum(rows,[]),'palettes':sum(prows,[]),'collision':sum(crows,[]),'events':events,'commands':commands,'boss':{'kind':BOSSES[s],'distance':float(11*240+238 if s<5 else 13*240+14)},'source':{'map_sequence':order,'collision_thresholds':thresholds,'world_scroll_per_event_page':240,'map_page_height':256,'streamed_page_height':240,'skipped_source_rows_per_page':2,'source_rows_reversed':True},'initial_camera_from_bottom':240,'start_camera_x':[0,128,68,0,144,205][s]})
 return stages

def extract_sprites(prg):
 d=prg[16384:32768]
 frames=[]
 for frame_id in range(256):
  a=(0x81f7 if frame_id<128 else 0x8b5c)+2*(frame_id&127)
  ptr=int.from_bytes(d[a-0x8000:a-0x8000+2],'little')
  count=d[ptr-0x8000] if 0x8000<=ptr<0xc000 else 0
  if count>64:
   frames.append({'id':frame_id,'parts':[],'unresolved':True});continue
  parts=[]
  for i in range(count):
   dy,tile,attr,dx=d[ptr-0x8000+1+i*4:ptr-0x8000+5+i*4]
   parts.append({'x':dx if dx<128 else dx-256,'y':dy if dy<128 else dy-256,'tile':tile,'palette':attr&3,'flip_x':bool(attr&64),'flip_y':bool(attr&128),'behind_background':bool(attr&32)})
  frames.append({'id':frame_id,'parts':parts})
 return {'sprite_width':8,'sprite_height':16,'frames':frames,'note':'Frames are original metasprites. Animation timing/state sequences are separate game logic.'}

def extract_entity_specs(prg):
 def data(a,n=1):
  bank=7 if a>=0xc000 else 6;base=0xc000 if bank==7 else 0x8000
  return prg[bank*16384+a-base:bank*16384+a-base+n]
 def word(a):return int.from_bytes(data(a,2),'little')
 specs=[]
 scores=[0,50,100,200,300,500,800,1000,1500,2000,3000,5000,8000,9000,9990,10]
 for kind in range(1,0x54):
  handler=word(0xf926+kind*2);hp=data(0xfa42+kind)[0];flags=data(0xf9ce+kind)[0];death=data(0xfa96+kind)[0]
  raw=data(handler,16);dispatch=raw.find(bytes([0x20,0x96,0xca]));states=[]
  if dispatch>=0:
   table=handler+dispatch+3
   # Original dispatch tables end at first forward local state body.
   end=table+32
   for i in range(16):
    if table+i*2>=end:break
    target=word(table+i*2)
    if not 0x8000<=target<0xffff:break
    states.append(target)
    if table<target<table+64:end=min(end,target)
  frame=None
  if states:
   raw=data(states[0],100)
   for i in range(len(raw)-4):
    if raw[i]==0xa9 and raw[i+2:i+5]==bytes([0x9d,0,5]):frame=raw[i+1];break
  box=list(data(0xfa22+(flags&15)*2,2))
  specs.append({'source_type':kind,'kind':KINDS.get(kind,f'entity_{kind:02x}'),'hit_points':hp&127,'grenade_only':bool(hp&128),'collision_flags':flags,'half_width':box[0],'half_height':box[1],'score':scores[death>>4],'death_state':death&15,'initial_frame_candidate':frame,'handler':f'{handler:04x}','state_handlers':[f'{x:04x}' for x in states],'behavior_recovery':'partial'})
 return {'specs':specs,'score_table':scores,'note':'All numeric metadata is original data; state handler lists are static candidates, not ported behavior.'}

def main():
 p=argparse.ArgumentParser();p.add_argument('--input',required=True,help='Historical reference data used only for offline extraction');p.add_argument('--out',default='assets/levels');a=p.parse_args()
 raw=Path(a.input).read_bytes();raw=raw[16:] if raw[:4]==b'NES\x1a' else raw
 out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
 for i,stage in enumerate(extract(raw),1):
  target=out/f'stage_{i:02d}.json';target.write_text(json.dumps(stage,separators=(',',':'))+'\n');print(target,stage['width_tiles'],stage['height_tiles'],len(stage['events']))
 specs=extract_entity_specs(raw)
 analysis=Path('analysis/native-entities.json');analysis.parent.mkdir(exist_ok=True)
 analysis.write_text(json.dumps(specs,indent=2)+'\n')
 runtime=[]
 for e in specs['specs']:
  t=e['source_type'];family=FAMILY_BY_ID.get(t,'effect')
  frame=FRAMES.get(t)
  runtime.append({'source_type':t,'kind':KINDS.get(t,f'effect_variant_{t:02x}'),'family':family,'hit_points':e['hit_points'],'grenade_only':e['grenade_only'],'half_width':e['half_width'],'half_height':e['half_height'],'score':e['score'],'animation':{'frames':[] if frame is None else [frame],'frame_ticks':8}})
 (out/'entity_specs.json').write_text(json.dumps({'specs':runtime},separators=(',',':'))+'\n')
 (out/'sprites.json').write_text(json.dumps(extract_sprites(raw),separators=(',',':'))+'\n')
if __name__=='__main__':main()
