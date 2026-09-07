export type WindowRole = "main" | "launcher" | "dock";

export function resolveWindowRole(label: string): WindowRole {
  if (label === "main" || label === "launcher" || label === "dock") return label;
  console.error("N1_Recipe: unknown window label; falling back to main UI", label);
  return "main";
}
