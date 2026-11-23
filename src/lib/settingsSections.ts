import { ArrowDownToLine, Bot, Database, Layers, LifeBuoy, Mail, Palette, Plug, Power, ScrollText, ServerCog, Settings2, Share2, SquareTerminal } from "lucide-react";

/** Settings → section index; ids are the anchors. Keywords are what the command palette also matches. */
export const sections = [
  { id: "general", label: "General", icon: Settings2, keywords: ["folder", "root", "domain", "tld", "suffix", "editor", "https", "hosts", "import", "setup guide"] },
  { id: "startup", label: "Startup", icon: Power, keywords: ["autostart", "sign in", "login", "tray", "launch", "boot"] },
  { id: "terminal", label: "Terminal", icon: SquareTerminal, keywords: ["shell", "path", "shims", "command line", "cli", "console"] },
  { id: "assistants", label: "AI assistants", icon: Bot, keywords: ["mcp", "claude", "cursor", "copilot", "codex", "agent"] },
  { id: "web", label: "Web server", icon: ServerCog, keywords: ["apache", "nginx", "caddy", "frankenphp", "vhost", "php workers"] },
  { id: "databases", label: "Databases", icon: Database, keywords: ["mysql", "mariadb", "postgres", "default database", "backup", "root password"] },
  { id: "versions", label: "Active versions", icon: Layers, keywords: ["php version", "switch version", "default version"] },
  { id: "ports", label: "Ports", icon: Plug, keywords: ["port", "80", "443", "3306", "conflict", "in use"] },
  { id: "email", label: "Email", icon: Mail, keywords: ["mail", "smtp", "mailpit", "sendmail"] },
  { id: "logs", label: "Logs", icon: ScrollText, keywords: ["log size", "rotate", "dump", "dd", "var_dump"] },
  { id: "sharing", label: "Sharing", icon: Share2, keywords: ["tunnel", "ngrok", "cloudflare", "public url", "webhook", "share"] },
  { id: "appearance", label: "Appearance", icon: Palette, keywords: ["theme", "dark mode", "light mode", "motion", "animation", "navigation bar"] },
  { id: "updates", label: "Updates", icon: ArrowDownToLine, keywords: ["update", "upgrade", "version", "release"] },
  { id: "help", label: "Troubleshooting", icon: LifeBuoy, keywords: ["diagnostics", "uninstall", "remove", "windows security", "defender", "antivirus", "slow composer"] },
] as const;
