@group(0) @binding(0) var<uniform> mvp: mat4x4<f32>;
struct Out { @builtin(position) p: vec4<f32>, @location(0) c: vec3<f32> };
@vertex fn vs(@location(0) p:vec3<f32>, @location(1) c:vec3<f32>) -> Out {
 var o:Out; o.p=mvp*vec4<f32>(p,1.0); o.c=c; return o;
}
@fragment fn fs(i:Out)->@location(0) vec4<f32>{ return vec4<f32>(i.c,1.0); }
