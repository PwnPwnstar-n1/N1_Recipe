export function extractVariables(text: string): string[] {
  const result: string[]=[]; const seen=new Set<string>();
  for (const match of text.matchAll(/\{\{([^{}]*)\}\}/gu)) {
    const name=match[1].trim();
    if (name && !["date","time","datetime","clipboard"].includes(name) && !seen.has(name)) { seen.add(name); result.push(name); }
  }
  return result;
}
export function renderTemplate(text:string, values:Record<string,string>):string {
  return text.replace(/\{\{([^{}]*)\}\}/gu,(token,raw:string)=>{
    const name=raw.trim(); return name && Object.prototype.hasOwnProperty.call(values,name) ? values[name] : token;
  });
}
