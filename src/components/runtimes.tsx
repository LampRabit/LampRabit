import type { ReactNode } from "react";
import { ArchiveRestore, Boxes, Braces, GitBranch, BarChart3, Bot, Cog, HardDrive, Inbox, Search, Workflow, Cloud, Code2, Coffee, Compass, Cpu, Feather as FeatherIcon, Flame, Gauge, Leaf, Network, SquareTerminal, Terminal, Database, DatabaseZap, Feather, Layers3, Mail, Package, Radio, Server, ShieldCheck, Table2, TableProperties, Zap } from "lucide-react";
import type { Runtime } from "../lib/api";

/** Service icons are neutral: colour is kept for status (the status light), not identity. */
export const TILE = "bg-surface-2 text-muted";

/** Icon used for each runtime across the app. */
export const runtimeTiles: Record<Runtime, { className: string; text: string; icon: (size: number) => ReactNode }> = {
  apache: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Feather size={size} strokeWidth={1.75} />,
  },
  nginx: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Server size={size} strokeWidth={1.75} />,
  },
  caddy: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <ShieldCheck size={size} strokeWidth={1.75} />,
  },
  frankenphp: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Flame size={size} strokeWidth={1.75} />,
  },
  tomcat: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Cpu size={size} strokeWidth={1.75} />,
  },
  java: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Coffee size={size} strokeWidth={1.75} />,
  },
  php: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Code2 size={size} strokeWidth={1.75} />,
  },
  mariadb: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Database size={size} strokeWidth={1.75} />,
  },
  mongodb: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Leaf size={size} strokeWidth={1.75} />,
  },
  meilisearch: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Search size={size} strokeWidth={1.75} />,
  },
  typesense: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Search size={size} strokeWidth={1.75} />,
  },
  rustfs: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <HardDrive size={size} strokeWidth={1.75} />,
  },
  minio: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <HardDrive size={size} strokeWidth={1.75} />,
  },
  elasticsearch: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Search size={size} strokeWidth={1.75} />,
  },
  opensearch: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Search size={size} strokeWidth={1.75} />,
  },
  n8n: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Workflow size={size} strokeWidth={1.75} />,
  },
  rabbitmq: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Inbox size={size} strokeWidth={1.75} />,
  },
  go: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Code2 size={size} strokeWidth={1.75} />,
  },
  rust: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Cog size={size} strokeWidth={1.75} />,
  },
  maven: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Package size={size} strokeWidth={1.75} />,
  },
  git: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <GitBranch size={size} strokeWidth={1.75} />,
  },
  bun: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Package size={size} strokeWidth={1.75} />,
  },
  deno: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Braces size={size} strokeWidth={1.75} />,
  },
  uv: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Zap size={size} strokeWidth={1.75} />,
  },
  conda: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Boxes size={size} strokeWidth={1.75} />,
  },
  gradle: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Layers3 size={size} strokeWidth={1.75} />,
  },
  erlang: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Cog size={size} strokeWidth={1.75} />,
  },
  ollama: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Bot size={size} strokeWidth={1.75} />,
  },
  mongotools: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <ArchiveRestore size={size} strokeWidth={1.75} />,
  },
  compass: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Compass size={size} strokeWidth={1.75} />,
  },
  sqlite: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <FeatherIcon size={size} strokeWidth={1.75} />,
  },
  clickhouse: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <BarChart3 size={size} strokeWidth={1.75} />,
  },
  neo4j: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Network size={size} strokeWidth={1.75} />,
  },
  mongosh: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <SquareTerminal size={size} strokeWidth={1.75} />,
  },
  python: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Terminal size={size} strokeWidth={1.75} />,
  },
  phppgadmin: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <TableProperties size={size} strokeWidth={1.75} />,
  },
  pgadmin: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Gauge size={size} strokeWidth={1.75} />,
  },
  postgres: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Database size={size} strokeWidth={1.75} />,
  },
  mysql: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <DatabaseZap size={size} strokeWidth={1.75} />,
  },
  mailpit: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Mail size={size} strokeWidth={1.75} />,
  },
  adminer: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Table2 size={size} strokeWidth={1.75} />,
  },
  redis: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Zap size={size} strokeWidth={1.75} />,
  },
  ngrok: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Radio size={size} strokeWidth={1.75} />,
  },
  cloudflared: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Cloud size={size} strokeWidth={1.75} />,
  },
  composer: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Package size={size} strokeWidth={1.75} />,
  },
  memcached: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <Layers3 size={size} strokeWidth={1.75} />,
  },
  phpredisadmin: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <TableProperties size={size} strokeWidth={1.75} />,
  },
  phpmyadmin: {
    className: TILE,
    text: "text-muted",
    icon: (size) => <TableProperties size={size} strokeWidth={1.75} />,
  },
};

/** A runtime's tile, or a plain one for a runtime the backend added that has none here yet. */
export function tileFor(id: string): { className: string; text: string; icon: (size: number) => ReactNode } {
  return (
    runtimeTiles[id as Runtime] ?? {
      className: TILE,
      text: "text-muted",
      icon: (size: number) => <Package size={size} strokeWidth={1.75} />,
    }
  );
}
