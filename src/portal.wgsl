@group(0) @binding(0) var blue:texture_2d<f32>;
@group(0) @binding(1) var orange:texture_2d<f32>;
@group(0) @binding(2) var<uniform> viewport:vec4<f32>;
struct Output {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) @interpolate(flat) index:u32,@location(2) @interpolate(flat) linked:f32};
@vertex fn vs(@location(0) position:vec4<f32>,@location(1) uv:vec2<f32>,@location(2) index:u32,@location(3) linked:f32)->Output {var o:Output;o.position=position;o.uv=uv;o.index=index;o.linked=linked;return o;}
@fragment fn fs(o:Output)->@location(0) vec4<f32> {
 let radius=dot(o.uv,o.uv);if radius>1. {discard;}
 let color=select(vec3<f32>(0.02,0.3,1.),vec3<f32>(1.,0.28,0.015),o.index==1u);
 // A lone authored portal has no destination: do not invent a remote view.
 // Procedural dormant-depth cue, not the original animated Source material.
 if radius>0.91 {return vec4<f32>(color,1.);}
 if o.linked<0.5 {
   let r=sqrt(radius);
   let angle=atan2(o.uv.y,o.uv.x);
   let spiral=0.5+0.5*sin(angle*3.+r*18.);
   let glow=0.035+0.24*r*r+0.055*spiral*r;
   return vec4<f32>(color*glow,1.);
 }
 let uv=o.position.xy/viewport.xy;
 if o.index==0u {let size=textureDimensions(blue);let xy=min(vec2<u32>(uv*vec2<f32>(size)),size-vec2<u32>(1));return textureLoad(blue,vec2<i32>(xy),0);}
 let size=textureDimensions(orange);let xy=min(vec2<u32>(uv*vec2<f32>(size)),size-vec2<u32>(1));return textureLoad(orange,vec2<i32>(xy),0);
}
