from pathlib import Path
root=Path('rust/crates/vexel-scene')
p=root/'src/types.rs'
s=p.read_text().replace(': pub ',': ')
s=s.replace('InvalidArgument,Unsupported','InvalidArgument,Xml(vexel_xml::XmlError),Unsupported')
s=s[:s.index('#[derive(Debug)] pub struct PatternRequest')] + s[s.index('#[derive(Debug, Clone, Copy, PartialEq)] pub struct Mask'):]
s += '''\nimpl ErrorKind { pub fn name(self)->&'static str { match self {
Self::InvalidArgument=>"invalid-argument",Self::Xml(_)=>"xml",Self::Unsupported=>"unsupported",Self::Number=>"number",Self::Path=>"path",Self::Viewport=>"viewport",Self::OutOfMemory=>"oom",Self::Canceled=>"canceled",Self::Limit(_)=>"limit",Self::UnsupportedFeTurbulence=>"unsupported-feTurbulence",Self::UnsupportedFeDisplacementMap=>"unsupported-feDisplacementMap",Self::UnsupportedFeConvolveMatrix=>"unsupported-feConvolveMatrix",Self::UnsupportedFeImage=>"unsupported-feImage",Self::UnsupportedFeLighting=>"unsupported-feLighting",Self::UnsupportedFeMorphology=>"unsupported-feMorphology"} } }
impl Limits { pub(crate) fn effective(self)->Self { let d=Self::default(); Self {
source_bytes:self.source_bytes.min(d.source_bytes),reserved_bytes:self.reserved_bytes,
scenes:self.scenes.min(d.scenes),shapes:self.shapes.min(d.shapes),edges:self.edges.min(d.edges),commands:self.commands.min(d.commands),numbers:self.numbers.min(d.numbers),transforms:self.transforms.min(d.transforms),clips:self.clips.min(d.clips),patterns:self.patterns.min(d.patterns),masks:self.masks.min(d.masks),filters:self.filters.min(d.filters),stops:self.stops.min(d.stops),groups:self.groups.min(d.groups)}} }
'''
p.write_text(s)
p=root/'Cargo.toml';p.write_text(p.read_text()+'vexel-xml = { path = "../vexel-xml" }\n')
tables=[('edges','Edge'),('edge_colors','[u8;4]'),('edge_shape_ids','u32'),('shapes','Shape'),('path_subpaths','vexel_svg_path::PathSubpath'),('path_segments','ScenePathSegment'),('path_numbers','vexel_svg_path::PathNumber'),('transforms','TransformFrame'),('images','Image'),('groups','OpacityGroup'),('filters','Filter'),('filter_primitives','FilterPrimitive'),('masks','Mask'),('patterns','Pattern'),('fill_paints','Paint'),('stroke_paints','Paint'),('dash_values','f64'),('stops','PaintStop'),('clips','Clip')]
handles=[('shapes','ShapeId'),('transforms','TransformId'),('clips','ClipId'),('patterns','PatternId'),('masks','MaskId'),('filters','FilterId'),('groups','GroupId')]
alltables=tables+[(n+'_serials','u64') for n,t in handles]
s='''use crate::*;
use crate::memory::{bytes,add};
/// Mutable storage is private to the builder. Native objects never enter this graph.
#[derive(Debug)]
pub(crate) struct SceneData {
pub(crate) serial:u64,pub(crate) sealed:bool,pub(crate) epoch:u64,pub(crate) viewport:Viewport,
'''
s+=''.join(f'pub(crate) {n}:Vec<{t}>,\n' for n,t in alltables)
s+='}\nimpl SceneData {pub(crate) fn new(serial:u64,viewport:Viewport)->Self{Self{serial,sealed:false,epoch:0,viewport,\n'
s+=''.join(f'{n}:Vec::new(),\n' for n,t in alltables)+'}}\n'
s+='pub(crate) fn lengths(&self)->[usize;26]{['+','.join('self.'+n+'.len()' for n,t in alltables)+']}\n'
s+='pub(crate) fn truncate(&mut self,lengths:[usize;26]){\n'+''.join(f'self.{n}.truncate(lengths[{i}]);\n' for i,(n,t) in enumerate(alltables))+'}\n'
s+='pub(crate) fn storage_bytes(&self)->Result<usize>{let mut total=0;\n'+''.join(f'total=add(total,bytes(&self.{n})?)?;\n' for n,t in alltables)
s+='''for image in &self.images {total=add(total,bytes(&image.pixels.rgba)?)?;}
for clip in &self.clips {total=add(total,add(bytes(&clip.geometry.edges)?,bytes(&clip.geometry.shapes)?)?)?;}
for pattern in &self.patterns {total=add(total,pattern.tile.allocated_byte_count())?;}
Ok(total)}
}
/// Borrowed immutable scene tables. Every range is validated before graph publication.
#[derive(Debug,Clone,Copy)]
pub struct SceneView<'a> {pub id:SceneId,pub viewport:Viewport,
'''
s+=''.join(f'pub {n}:&\'a [{t}],\n' for n,t in tables)
s+='''serials: [&'a [u64];7],
}
impl SceneData {pub(crate) fn view(&self,owner:u64,slot:usize)->SceneView<'_>{SceneView {
id:SceneId{owner,slot:slot as u32,serial:self.serial},viewport:self.viewport,
'''
s+=''.join(f'{n}:&self.{n},\n' for n,t in tables)
s+='serials:['+','.join('&self.'+n+'_serials' for n,t in handles)+'],}}}\n'
s+='impl SceneView<\'_> {\n'
for i,(n,t) in enumerate(handles):
 s+=f'''pub fn {t[:-2].lower()}(&self,id:{t})->Option<&{dict(tables)[n]}>{{
if id.owner!=self.id.owner||id.scene!=self.id.slot||self.serials[{i}].get(id.index())!=Some(&id.serial){{return None;}} self.{n}.get(id.index())}}
'''
s+='''pub fn edge_shape(&self,edge:usize)->Option<ShapeId>{let slot=*self.edge_shape_ids.get(edge)?;Some(ShapeId{owner:self.id.owner,scene:self.id.slot,slot,serial:*self.serials[0].get(slot as usize)?})}
}
/// Immutable, source-coherent scene graph suitable for worker ownership transfer.
#[derive(Debug)]
pub struct SceneGraph {pub(crate) owner:u64,pub(crate) source:SourceOwner,pub(crate) generation:u64,pub(crate) root:SceneId,pub(crate) scenes:Vec<SceneData>,pub(crate) animation:OwnedAnimation,pub(crate) ignored:[Ignored;4],pub(crate) stats:MemoryStats}
impl SceneGraph {
pub fn source_owner(&self)->&SourceOwner{&self.source}
pub fn source(&self)->&[u8]{self.source.as_slice()}
pub fn generation(&self)->u64{self.generation}
pub fn root(&self)->SceneView<'_>{self.scene(self.root).expect("validated root")}
pub fn scene(&self,id:SceneId)->Option<SceneView<'_>>{let scene=self.scenes.get(id.index())?;if id.owner!=self.owner||id.serial!=scene.serial{return None;}Some(scene.view(self.owner,id.index()))}
pub fn scene_count(&self)->usize{self.scenes.len()}
pub fn animation(&self)->AnimationView<'_>{self.animation.view()}
pub fn ignored(&self,kind:IgnoredKind)->Ignored{self.ignored[kind as usize]}
pub fn memory_stats(&self)->MemoryStats{self.stats}
}
'''
(root/'src/model.rs').write_text(s)
