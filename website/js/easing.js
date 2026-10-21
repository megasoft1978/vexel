(()=>{
const pad=document.getElementById("pad");if(!pad)return;
const S=240,M=20,ns="http://www.w3.org/2000/svg";
const p=[.42,0,.58,1],out=document.getElementById("ease-out"),cv=document.getElementById("cv"),dot=document.getElementById("dot");
const hs=[...pad.querySelectorAll(".hd")],ls=[...pad.querySelectorAll(".hl")];
const X=v=>M+v*S,Y=v=>M+S-v*S,clamp=(v,a,b)=>Math.min(b,Math.max(a,v));
const bz=(t,a,b)=>3*(1-t)*(1-t)*t*a+3*(1-t)*t*t*b+t*t*t;
function ease(x){let lo=0,hi=1,t=x;for(let i=0;i<24;i++){t=(lo+hi)/2;bz(t,p[0],p[2])<x?lo=t:hi=t}return bz(t,p[1],p[3])}
function draw(){
 cv.setAttribute("d",`M${X(0)} ${Y(0)}C${X(p[0])} ${Y(p[1])} ${X(p[2])} ${Y(p[3])} ${X(1)} ${Y(1)}`);
 hs.forEach((h,i)=>{const x=X(p[i*2]),y=Y(p[i*2+1]);h.setAttribute("cx",x);h.setAttribute("cy",y);
  h.setAttribute("aria-valuenow",p[i*2].toFixed(2));h.setAttribute("aria-valuetext",`x ${p[i*2].toFixed(2)}, y ${p[i*2+1].toFixed(2)}`);
  ls[i].setAttribute("x1",X(i));ls[i].setAttribute("y1",Y(i));ls[i].setAttribute("x2",x);ls[i].setAttribute("y2",y)});
 out.textContent=`cubic-bezier(${p.map(v=>v.toFixed(2)).join(", ")})`;
}
function set(i,x,y){p[i*2]=clamp(x,0,1);p[i*2+1]=clamp(y,-.5,1.5);draw()}
hs.forEach((h,i)=>{
 const pt=e=>{const r=pad.getBoundingClientRect(),k=(S+2*M)/r.width;return[((e.clientX-r.left)*k-M)/S,1-((e.clientY-r.top)*k-M)/S]};
 h.addEventListener("pointerdown",e=>{h.setPointerCapture(e.pointerId);h.focus();e.preventDefault()});
 h.addEventListener("pointermove",e=>{if(h.hasPointerCapture(e.pointerId)){const[x,y]=pt(e);set(i,x,y)}});
 h.addEventListener("keydown",e=>{const d={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,1],ArrowDown:[0,-1]}[e.key];if(!d)return;
  const s=e.shiftKey?.1:.01;set(i,p[i*2]+d[0]*s,p[i*2+1]+d[1]*s);e.preventDefault()});
});
document.querySelectorAll(".pre button").forEach(b=>b.addEventListener("click",()=>{
 const v=b.dataset.v.split(",").map(Number);v.forEach((n,j)=>p[j]=n);draw()}));
const rm=matchMedia("(prefers-reduced-motion:reduce)");
let t0=0;
function place(u){const e=ease(u);dot.setAttribute("cx",X(u));dot.setAttribute("cy",Y(e))}
function tick(ts){if(!t0)t0=ts;place(((ts-t0)%2400)/2000>1?1:((ts-t0)%2400)/2000);requestAnimationFrame(tick)}
draw();rm.matches?place(1):requestAnimationFrame(tick);
})();
