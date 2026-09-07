import {availableMonitors,getCurrentWindow,PhysicalPosition,PhysicalSize} from "@tauri-apps/api/window";
type Geometry={x:number;y:number;width:number;height:number};
export async function restoreAndTrackWindow(label:string):Promise<()=>void>{
  if(label==="dock")return()=>{};const win=getCurrentWindow();const key=`snipdeck.geometry.${label}`;
  try{const raw=localStorage.getItem(key);if(raw){const g=JSON.parse(raw) as Geometry;const monitors=await availableMonitors();const monitor=monitors.find(m=>g.x+80>=m.position.x&&g.x+80<m.position.x+m.size.width&&g.y+40>=m.position.y&&g.y+40<m.position.y+m.size.height)??monitors[0];if(monitor){const width=Math.min(g.width,monitor.size.width),height=Math.min(g.height,monitor.size.height);const x=Math.max(monitor.position.x,Math.min(g.x,monitor.position.x+monitor.size.width-width));const y=Math.max(monitor.position.y,Math.min(g.y,monitor.position.y+monitor.size.height-height));await win.setSize(new PhysicalSize(width,height));await win.setPosition(new PhysicalPosition(x,y));}}}catch{/* invalid old geometry is ignored */}
  let timer:number|undefined;const save=()=>{window.clearTimeout(timer);timer=window.setTimeout(async()=>{try{const [p,s]=await Promise.all([win.outerPosition(),win.innerSize()]);localStorage.setItem(key,JSON.stringify({x:p.x,y:p.y,width:s.width,height:s.height} satisfies Geometry))}catch{}},250)};
  const [u1,u2]=await Promise.all([win.onMoved(save),win.onResized(save)]);return()=>{u1();u2();window.clearTimeout(timer)};
}
