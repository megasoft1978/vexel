(()=>{
const rm=matchMedia("(prefers-reduced-motion:reduce)");
/* reduced motion: stop autoplay, show poster */
document.querySelectorAll("video[data-loop]").forEach(v=>{if(rm.matches){v.removeAttribute("autoplay");v.pause()}});
if(rm.matches)document.documentElement.classList.add("nomo");
/* start log when visible */
const log=document.querySelector(".log");
if(log&&"IntersectionObserver"in window){new IntersectionObserver((es,o)=>{es.forEach(e=>{if(e.isIntersecting){log.classList.add("on");o.disconnect()}})},{threshold:.3}).observe(log)}else if(log)log.classList.add("on");
/* tabs */
const tabs=[...document.querySelectorAll("[role=tab]")];
function sel(t){tabs.forEach(x=>{const on=x===t;x.setAttribute("aria-selected",on);x.tabIndex=on?0:-1;document.getElementById(x.getAttribute("aria-controls")).hidden=!on});t.focus()}
tabs.forEach((t,i)=>{t.addEventListener("click",()=>sel(t));
 t.addEventListener("keydown",e=>{const n={ArrowRight:i+1,ArrowLeft:i-1,Home:0,End:tabs.length-1}[e.key];if(n===undefined)return;e.preventDefault();sel(tabs[(n+tabs.length)%tabs.length])})});
/* copy */
document.querySelectorAll(".copy").forEach(b=>b.addEventListener("click",async()=>{
 const txt=b.parentElement.querySelector("pre").innerText;
 try{await navigator.clipboard.writeText(txt);b.textContent="Copied"}catch(_){const r=document.createRange();r.selectNodeContents(b.parentElement.querySelector("pre"));getSelection().removeAllRanges();getSelection().addRange(r);b.textContent="Selected"}
 setTimeout(()=>b.textContent="Copy",1800)}));
/* demo: no autoplay */
})();
