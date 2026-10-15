import struct,gzip,base64
from pathlib import Path
import numpy as np
root=Path(__file__).parent
b=(root/'scene.bin').read_bytes();nv,nt,nb=struct.unpack_from('<III',b);o=28
v=np.frombuffer(b,dtype='<f4',count=nv*12,offset=o).copy().reshape(-1,12);o+=nv*48
v[:,:3]=np.round(v[:,:3]*64)/64
v[:,3:7]=np.round(v[:,3:7]*4096)/4096
v[:,9:]=np.round(v[:,9:]*256)/256
tex=np.frombuffer(b,dtype=np.uint8,count=nt*256*256*4,offset=o).reshape(nt,256,256,4);o+=tex.size
tex=tex[:,::2,::2,:].copy()
# Unsupported refract checkerboard is not a real glass texture. Transparent fallback.
for t in tex:
 if (t[:,:,0]==255).mean()>.99 and (t[:,:,2]==255).mean()>.99 and t[:,:,1].mean()<100:t[:,:,3]=0
light=np.frombuffer(b,dtype=np.uint8,count=4096*4096*4,offset=o).reshape(4096,4096,4);o+=light.size
packed=b[:28]+v.tobytes()+tex.tobytes()+light[::2,::2,:].tobytes()+b[o:]
data=base64.b64encode(gzip.compress(packed,9)).decode()
html='''<!doctype html><meta charset="utf-8"><title>Portal · Chamber 00 / offline</title><style>
*{box-sizing:border-box}body{margin:0;background:#111;color:#eee;font:14px system-ui;-webkit-font-smoothing:antialiased}canvas{display:block;width:100vw;height:100vh}#hud{position:fixed;left:24px;top:20px;padding:14px 18px;background:#111b;border-radius:12px;pointer-events:none}h1{font-size:18px;margin:0 0 8px}#status{color:#8ed9ee}button{position:fixed;bottom:24px;left:24px;border:1px solid #8ed9ee;background:#15252b;color:white;padding:14px 24px;border-radius:8px;cursor:pointer}button:active{transform:scale(.96)}#cross{position:fixed;top:50%;left:50%;color:#fff9;font-size:22px;pointer-events:none}</style><canvas id="c"></canvas><div id="hud"><h1>APERTURE / CHAMBER 00</h1><div>WASD walk · mouse look · Space jump · Shift sprint</div><div>N free-flight · R reset · Esc release mouse</div><div id="status">Unpacking embedded chamber…</div><small>Personal asset demo · WebGL2 · no portal gun or puzzle scripting</small></div><button id="start">Click to explore</button><div id="cross">+</div><script>
const DATA='''+repr(data)+''';
const status=document.querySelector('#status');window.errors=[];addEventListener('error',e=>{errors.push(e.message);status.textContent=e.message});
(async()=>{
const bytes=Uint8Array.from(atob(DATA),c=>c.charCodeAt(0));const buf=await new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'))).arrayBuffer();
const dv=new DataView(buf);let o=0;function u(){let x=dv.getUint32(o,true);o+=4;return x}function f(){let x=dv.getFloat32(o,true);o+=4;return x}
const nv=u(),nt=u(),nb=u(),spawn=[f(),f(),f()],initialYaw=f()+Math.PI;const verts=new Float32Array(buf,o,nv*12);o+=nv*48;const tex=new Uint8Array(buf,o,nt*128*128*4);o+=tex.length;const light=new Uint8Array(buf,o,2048*2048*4);o+=light.length;
const brushes=[];for(let j=0;j<nb;j++){let n=u(),p=[];for(let i=0;i<n;i++)p.push([f(),f(),f(),f()]);brushes.push(p)}
const canvas=document.querySelector('#c'),gl=canvas.getContext('webgl2',{antialias:true});if(!gl)throw Error('WebGL2 required');
function shader(type,src){const s=gl.createShader(type);gl.shaderSource(s,src);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s}
const prog=gl.createProgram();gl.attachShader(prog,shader(gl.VERTEX_SHADER,`#version 300 es
precision highp float;layout(location=0)in vec3 pos;layout(location=1)in vec2 uv;layout(location=2)in vec2 luv;layout(location=3)in float layer;layout(location=4)in float unlit;layout(location=5)in vec3 normal;
uniform vec3 eye,forward,right,up;uniform float aspect;out vec2 t,l;flat out float a;out float un;
void main(){vec3 d=pos-eye;float z=dot(d,forward);gl_Position=vec4(dot(d,right)/aspect,dot(d,up),1.0002*z-2.0002,z);t=uv;l=luv;a=layer;un=unlit;}`));gl.attachShader(prog,shader(gl.FRAGMENT_SHADER,`#version 300 es
precision highp float;precision highp sampler2DArray;in vec2 t,l;flat in float a;in float un;uniform sampler2DArray base;uniform sampler2D lm;out vec4 color;
void main(){vec4 b=texture(base,vec3(t,a));if(b.a<.12)discard;vec3 irradiance=max(texture(lm,l).rgb*4.,vec3(.18));float fullbright=(un>.5&&un<1.5)?1.:0.;color=vec4(pow(pow(b.rgb,vec3(2.2))*mix(irradiance,vec3(1.),fullbright),vec3(1./2.2)),b.a);}`));gl.linkProgram(prog);if(!gl.getProgramParameter(prog,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(prog));gl.useProgram(prog);
const vb=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,vb);gl.bufferData(gl.ARRAY_BUFFER,verts,gl.STATIC_DRAW);[[0,3,0],[1,2,12],[2,2,20],[3,1,28],[4,1,32],[5,3,36]].forEach(([a,n,o])=>{gl.enableVertexAttribArray(a);gl.vertexAttribPointer(a,n,gl.FLOAT,false,48,o)});
function texture(unit,target,w,h,depth,pixels){gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(target,gl.createTexture());if(depth)gl.texImage3D(target,0,gl.RGBA8,w,h,depth,0,gl.RGBA,gl.UNSIGNED_BYTE,pixels);else gl.texImage2D(target,0,gl.RGBA8,w,h,0,gl.RGBA,gl.UNSIGNED_BYTE,pixels);gl.texParameteri(target,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(target,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(target,gl.TEXTURE_WRAP_S,depth?gl.REPEAT:gl.CLAMP_TO_EDGE);gl.texParameteri(target,gl.TEXTURE_WRAP_T,depth?gl.REPEAT:gl.CLAMP_TO_EDGE)}
texture(0,gl.TEXTURE_2D_ARRAY,128,128,nt,tex);texture(1,gl.TEXTURE_2D,2048,2048,0,light);gl.uniform1i(gl.getUniformLocation(prog,'base'),0);gl.uniform1i(gl.getUniformLocation(prog,'lm'),1);gl.enable(gl.DEPTH_TEST);
const keys=new Set();let pos=spawn.slice(),yaw=initialYaw,pitch=0,vz=0,noclip=false,ground=false,frames=0;const dot=(a,b)=>a[0]*b[0]+a[1]*b[1]+a[2]*b[2];
function overlaps(p){return brushes.some(b=>b.every(n=>n[0]*p[0]+n[1]*p[1]+n[2]*(p[2]+36)-n[3]-16*Math.abs(n[0])-16*Math.abs(n[1])-36*Math.abs(n[2])<-.01))}
function reset(){pos=spawn.slice();yaw=initialYaw;pitch=0;vz=0}window.demo={get position(){return pos.slice()},get yaw(){return yaw},get frames(){return frames},get noclip(){return noclip},get grounded(){return ground},overlaps,reset,keys,spawn,vertices:nv,brushes:nb};
addEventListener('keydown',e=>{if(['Space','ArrowUp','ArrowDown'].includes(e.code))e.preventDefault();keys.add(e.code);if(!e.repeat&&e.code==='KeyN')noclip=!noclip;if(e.code==='KeyR')reset()});addEventListener('keyup',e=>keys.delete(e.code));addEventListener('blur',()=>keys.clear());document.querySelector('#start').onclick=()=>canvas.requestPointerLock();canvas.onclick=()=>canvas.requestPointerLock();document.addEventListener('pointerlockchange',()=>{document.querySelector('#start').style.display=document.pointerLockElement?'none':'block';keys.clear()});addEventListener('mousemove',e=>{if(document.pointerLockElement===canvas){yaw-=e.movementX*.0025;pitch=Math.max(-1.5,Math.min(1.5,pitch-e.movementY*.0025))}});
let last=performance.now();function tick(now){let dt=Math.min((now-last)/1000,.04);last=now;let fw=[Math.cos(yaw),Math.sin(yaw),0],rt=[Math.sin(yaw),-Math.cos(yaw),0];let a=(keys.has('KeyW')?1:0)-(keys.has('KeyS')?1:0),b=(keys.has('KeyD')?1:0)-(keys.has('KeyA')?1:0),speed=keys.has('ShiftLeft')?280:175;let dx=(fw[0]*a+rt[0]*b)*speed*dt,dy=(fw[1]*a+rt[1]*b)*speed*dt;
if(noclip){pos[0]+=dx;pos[1]+=dy;pos[2]+=((keys.has('Space')?1:0)-(keys.has('ControlLeft')?1:0))*speed*dt;vz=0}else{const steps=Math.max(1,Math.ceil(Math.hypot(dx,dy)/4));for(let i=0;i<steps;i++){for(let axis=0;axis<2;axis++){let p=pos.slice();p[axis]+=(axis?dy:dx)/steps;if(!overlaps(p))pos=p;else{p[2]+=18;if(!overlaps(p))pos=p}}}ground=overlaps([pos[0],pos[1],pos[2]-2]);if(ground&&keys.has('Space'))vz=260;vz-=600*dt;let dz=vz*dt;for(let i=0,n=Math.max(1,Math.ceil(Math.abs(dz)/2));i<n;i++){let p=pos.slice();p[2]+=dz/n;if(!overlaps(p))pos=p;else{vz=0;break}}}if(pos[2]<-4000)reset();
const scale=Math.min(devicePixelRatio,1.5);let w=Math.floor(innerWidth*scale),h=Math.floor(innerHeight*scale);if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h}gl.viewport(0,0,w,h);gl.clearColor(.025,.035,.04,1);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);let forward=[Math.cos(yaw)*Math.cos(pitch),Math.sin(yaw)*Math.cos(pitch),Math.sin(pitch)],up=[-Math.cos(yaw)*Math.sin(pitch),-Math.sin(yaw)*Math.sin(pitch),Math.cos(pitch)];for(const [name,val]of Object.entries({eye:[pos[0],pos[1],pos[2]+64],forward,right:rt,up}))gl.uniform3fv(gl.getUniformLocation(prog,name),val);gl.uniform1f(gl.getUniformLocation(prog,'aspect'),w/h);gl.drawArrays(gl.TRIANGLES,0,nv);frames++;if(frames%30===0)status.textContent=(noclip?'FREE FLIGHT':'WALK / COLLISION')+' · '+pos.map(x=>Math.round(x)).join(' / ');requestAnimationFrame(tick)}requestAnimationFrame(tick);status.textContent='Ready · click to capture mouse';window.ready=true;
})().catch(e=>{errors.push(String(e));status.textContent=String(e)});
</script>'''
(root/'chamber00.html').write_text(html)
print('artifact bytes',len(html.encode()),'gzip scene bytes',len(data)*3//4)
assert len(html.encode())<9_000_000
