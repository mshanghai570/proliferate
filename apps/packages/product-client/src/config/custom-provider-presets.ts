export const CUSTOM_PROVIDER_PRESETS = [
  { id: "custom", label: "Custom endpoint", name: "", endpoint: "" },
  { id: "ollama", label: "Ollama", name: "Ollama", endpoint: "http://localhost:11434/v1" },
  { id: "lmstudio", label: "LM Studio", name: "LM Studio", endpoint: "http://localhost:1234/v1" },
] as const;
