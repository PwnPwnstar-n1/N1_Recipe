export type Theme="system"|"light"|"dark"; export type FontSize="small"|"standard"|"large"|"xlarge";
const media=window.matchMedia("(prefers-color-scheme: dark)");
export function applyAppearance(theme:Theme=(localStorage.getItem("snipdeck.theme") as Theme)||"system",font:FontSize=(localStorage.getItem("snipdeck.font") as FontSize)||"standard"){
  const resolved=theme==="system"?(media.matches?"dark":"light"):theme;
  document.documentElement.dataset.theme=resolved;document.documentElement.dataset.font=font;
  localStorage.setItem("snipdeck.theme",theme);localStorage.setItem("snipdeck.font",font);
}
media.addEventListener("change",()=>{if((localStorage.getItem("snipdeck.theme")||"system")==="system")applyAppearance()});
