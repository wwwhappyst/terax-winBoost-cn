const LABELS: Record<string, string> = {
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini",
  grok: "Grok",
  opencode: "OpenCode",
  kimi: "Kimi",
  terax: "Terax",
};

export function displayAgent(agent: string): string {
  if (!agent) return "Agent";
  return LABELS[agent.toLowerCase()] ?? agent.charAt(0).toUpperCase() + agent.slice(1);
}
