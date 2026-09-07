import {describe,expect,it} from "vitest";
import {extractVariables,renderTemplate} from "./template";
describe("Unicode template variables",()=>{
  it("extracts, trims and deduplicates",()=>expect(extractVariables("{{name}} {{ 文章路径 }} {{项目 名称}} {{目标-IP}} {{文章路径}} {{}}" )).toEqual(["name","文章路径","项目 名称","目标-IP"]));
  it("replaces every occurrence literally",()=>expect(renderTemplate("{{x}}/{{ x }}",{x:'$1\\"C:'})).toBe('$1\\"C:/$1\\"C:'));
  it("leaves braces alone",()=>expect(renderTemplate("{x} } {{}}",{})).toBe("{x} } {{}}"));
});
