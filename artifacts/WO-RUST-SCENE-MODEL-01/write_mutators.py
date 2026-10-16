from pathlib import Path
p=Path('rust/crates/vexel-scene/src/mutators.rs')
s='''use crate::*;
use crate::builder::room;
use crate::memory::{add,bytes};
use crate::types::{invalid,limit,stopped};
impl SceneBuilder {
'''
items=[('transform','transforms','TransformFrame','TransformId','transforms','Transforms','0'),('clip','clips','Clip','ClipId','clips','Clips','add(bytes(&value.geometry.edges)?,bytes(&value.geometry.shapes)?)?'),('pattern','patterns','Pattern','PatternId','patterns','Patterns','value.tile.allocated_byte_count()'),('mask','masks','Mask','MaskId','masks','Masks','0'),('group','groups','OpacityGroup','GroupId','groups','Nodes','0')]
for name,table,ty,handle,cap,lim,nested in items:
 s+=f'''pub fn append_{name}(&mut self,scene:SceneId,value:{ty},control:&mut dyn WorkControl)->Result<{handle}>{{
let i=self.writable(scene)?;control.checkpoint().map_err(stopped)?;
crate::validate::{name}(self,scene,&value,control)?;
if self.scenes[i].{table}.len()>=self.limits.{cap}{{return Err(limit(LimitKind::{lim}));}}
let nested={nested};self.pending(nested)?;let serial=self.serial()?;
room!(self,i,{table},1,self.limits.{cap},nested);
room!(self,i,{table}_serials,1,self.limits.{cap},nested);
control.checkpoint().map_err(stopped)?;self.retain(nested)?;
let id={handle}{{owner:self.owner,scene:scene.slot,slot:self.scenes[i].{table}.len() as u32,serial}};
self.scenes[i].{table}.push(value);self.scenes[i].{table}_serials.push(serial);Ok(id)}}
'''
s+='''pub fn append_image(&mut self,scene:SceneId,value:Image,control:&mut dyn WorkControl)->Result<usize>{
let i=self.writable(scene)?;control.checkpoint().map_err(stopped)?;crate::validate::image(&value,control)?;
let pixels=self.scenes[i].images.iter().try_fold(0usize,|sum,v|add(sum,(v.pixels.width as usize)*(v.pixels.height as usize)))?;
if add(pixels,(value.pixels.width as usize)*(value.pixels.height as usize))?>67_108_864{return Err(limit(LimitKind::Images));}
let nested=bytes(&value.pixels.rgba)?;self.pending(nested)?;
room!(self,i,images,1,self.limits.shapes,nested);control.checkpoint().map_err(stopped)?;self.retain(nested)?;
let index=self.scenes[i].images.len();self.scenes[i].images.push(value);Ok(index)}
pub fn append_stops(&mut self,scene:SceneId,values:&[PaintStop],control:&mut dyn WorkControl)->Result<TableRange>{
let i=self.writable(scene)?;if values.len()>256{return Err(limit(LimitKind::PaintStops));}
crate::validate::stops(values,control)?;
let cap=self.limits.stops;if add(self.scenes[i].stops.len(),values.len())?>cap{return Err(limit(LimitKind::PaintStops));}
room!(self,i,stops,values.len(),cap,0);let mark=self.mark(scene)?;let first=self.scenes[i].stops.len();
for value in values {if let Err(stop)=control.charge(WorkKind::PaintVisit,1){self.rollback(mark)?;return Err(stopped(stop));}self.scenes[i].stops.push(*value);}
Ok(TableRange{first,count:values.len()})}
pub fn append_dashes(&mut self,scene:SceneId,values:&[f64],control:&mut dyn WorkControl)->Result<TableRange>{
let i=self.writable(scene)?;if values.len()>16{return Err(limit(LimitKind::DashCache));}
for value in values {control.charge(WorkKind::StrokeTest,1).map_err(stopped)?;if !value.is_finite()||*value<0.0||*value>1e6{return Err(invalid(0));}}
let cap=self.limits.shapes.checked_mul(16).ok_or_else(crate::memory::arithmetic)?;
room!(self,i,dash_values,values.len(),cap,0);let mark=self.mark(scene)?;let first=self.scenes[i].dash_values.len();
for value in values {if let Err(stop)=control.charge(WorkKind::StrokeTest,1){self.rollback(mark)?;return Err(stopped(stop));}self.scenes[i].dash_values.push(*value);}
Ok(TableRange{first,count:values.len()})}
'''
for table in ['fill_paints','stroke_paints']:
 name='fill_paint' if table.startswith('fill') else 'stroke_paint'
 s+=f'''pub fn append_{name}(&mut self,scene:SceneId,value:Paint,control:&mut dyn WorkControl)->Result<usize>{{
let i=self.writable(scene)?;control.checkpoint().map_err(stopped)?;crate::validate::paint(self,scene,&value,control)?;
room!(self,i,{table},1,self.limits.shapes,0);control.checkpoint().map_err(stopped)?;
let index=self.scenes[i].{table}.len();self.scenes[i].{table}.push(value);Ok(index)}}
'''
s+='''pub fn append_filter(&mut self,scene:SceneId,mut value:Filter,primitives:&[FilterPrimitive],control:&mut dyn WorkControl)->Result<FilterId>{
let i=self.writable(scene)?;crate::validate::filter(value,primitives,control)?;
if self.scenes[i].filters.len()>=self.limits.filters{return Err(limit(LimitKind::Filters));}
let serial=self.serial()?;room!(self,i,filters,1,self.limits.filters,0);room!(self,i,filters_serials,1,self.limits.filters,0);
room!(self,i,filter_primitives,primitives.len(),65536,0);
let mark=self.mark(scene)?;value.primitives=TableRange{first:self.scenes[i].filter_primitives.len(),count:primitives.len()};
for primitive in primitives {if let Err(stop)=control.charge(WorkKind::FilterPixel,1){self.rollback(mark)?;return Err(stopped(stop));}self.scenes[i].filter_primitives.push(*primitive);}
let id=FilterId{owner:self.owner,scene:scene.slot,slot:self.scenes[i].filters.len() as u32,serial};
self.scenes[i].filters.push(value);self.scenes[i].filters_serials.push(serial);Ok(id)}
}
'''
p.write_text(s)
