struct Camera { matrix: mat4x4<f32>, clip:vec4<f32>, };
struct Models { offsets: array<vec4<f32>,256>, };
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> models: Models;
@group(0) @binding(2) var base: texture_2d_array<f32>;
@group(0) @binding(3) var light: texture_2d<f32>;
@group(0) @binding(4) var repeat_sampler: sampler;
@group(0) @binding(5) var clamp_sampler: sampler;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) light_uv: vec2<f32>, @location(2) @interpolate(flat) layer: u32, @location(3) unlit: f32, @location(4) normal: vec3<f32>, @location(5) world: vec3<f32>, };
@vertex fn vs(@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>,@location(2) light_uv:vec2<f32>,@location(3) layer:u32,@location(4) model:u32,@location(5) unlit:f32,@location(6) normal:vec3<f32>)->Output {
 var o: Output; o.world=position+models.offsets[model].xyz; o.position=camera.matrix*vec4<f32>(position+models.offsets[model].xyz,1.);if models.offsets[model].w>0.5 {o.position=vec4<f32>(2.,2.,2.,1.);}
 o.uv=uv;o.light_uv=light_uv;o.layer=layer;o.unlit=unlit;o.normal=normal;return o;
}
fn shade(o:Output, blended:bool)->vec4<f32> {
 if (o.unlit<0.) != blended {discard;}
 let lighting=select(o.unlit,-o.unlit-1.,blended);
 if dot(camera.clip,vec4<f32>(o.world,1.))<0. {discard;}
 let diffuse=textureSample(base,repeat_sampler,o.uv,i32(o.layer));
 if !blended && diffuse.a<0.4 {discard;}
 var irradiance=textureSample(light,clamp_sampler,o.light_uv).rgb;
 if lighting>2.5 {
   let n=normalize(o.normal); let weights=n*n;
   let step=3.0/f32(textureDimensions(light).x);
   let px=textureSample(light,clamp_sampler,o.light_uv+vec2<f32>(select(step,0.,n.x>=0.),0.)).rgb;
   let py=textureSample(light,clamp_sampler,o.light_uv+vec2<f32>(select(3.*step,2.*step,n.y>=0.),0.)).rgb;
   let pz=textureSample(light,clamp_sampler,o.light_uv+vec2<f32>(select(5.*step,4.*step,n.z>=0.),0.)).rgb;
   irradiance=px*weights.x+py*weights.y+pz*weights.z;
 }
 else if lighting>1.5 {irradiance=vec3<f32>(0.35+0.65*max(dot(normalize(o.normal),normalize(vec3<f32>(0.3,-0.5,0.8))),0.));}
 else if lighting>0.5 {irradiance=vec3<f32>(1.);}

 return vec4<f32>(diffuse.rgb*irradiance,select(1.,diffuse.a,blended));
}
@fragment fn fs(o:Output)->@location(0) vec4<f32> {return shade(o,false);}
@fragment fn fs_blend(o:Output)->@location(0) vec4<f32> {return shade(o,true);}
// Original combineball UnlitGeneric additive layers and Modulate MOD2X shell.
@fragment fn fs_add(o:Output)->@location(0) vec4<f32> {
 if o.unlit == -3. || dot(camera.clip,vec4<f32>(o.world,1.))<0. {discard;}
 let t=textureSample(base,repeat_sampler,o.uv,i32(o.layer));
 return vec4<f32>(t.rgb*select(1.,0.25,o.unlit == -4.),t.a);
}
@fragment fn fs_mod(o:Output)->@location(0) vec4<f32> {
 if o.unlit != -3. || dot(camera.clip,vec4<f32>(o.world,1.))<0. {discard;}
 // Modulate uses linear texture reads: neutral authored 128 must not darken a square.
 let t=textureSample(base,repeat_sampler,o.uv,i32(o.layer));
 let raw=select(12.92*t.rgb,1.055*pow(t.rgb,vec3<f32>(1./2.4))-vec3<f32>(0.055),t.rgb>vec3<f32>(0.0031308));
 return vec4<f32>(mix(vec3<f32>(1.),2.*raw,t.a),1.);
}
