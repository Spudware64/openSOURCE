@group(0) @binding(0) var blue:texture_2d<f32>;
@group(0) @binding(1) var orange:texture_2d<f32>;
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) @interpolate(flat) index:u32,@location(2) @interpolate(flat) linked:f32};
@vertex fn vs(@location(0) position:vec4<f32>,@location(1) uv:vec2<f32>,@location(2) index:u32,@location(3) linked:f32)->Output {var o:Output;o.position=position;o.uv=uv;o.index=index;o.linked=linked;return o;}
@fragment fn fs(o:Output)->@location(0) vec4<f32> {
 let radius=dot(o.uv,o.uv);if radius>1. {discard;}
 let color=select(vec3<f32>(0.02,0.3,1.),vec3<f32>(1.,0.28,0.015),o.index==1u);
 if radius>0.91 || o.linked<0.5 {return vec4<f32>(color,1.);}
 let xy=vec2<i32>(o.position.xy);
 if o.index==0u {return textureLoad(blue,xy,0);}return textureLoad(orange,xy,0);
}
