/**
 * esbuild plugin that applies the SWC durable functions transform per-file.
 */

import { transform } from "@swc/core";
import { resolve, dirname } from "node:path";
import { existsSync, promises as fsp } from "node:fs";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export interface DurablePluginOptions {
  /** Glob patterns for workflow files (transformed in workflow mode) */
  workflowPatterns?: string[];
  /** Glob patterns for client files (transformed in client mode) */
  clientPatterns?: string[];
  /** Package name for SDK import. Default: "@cgalceran/aws-durable" */
  packageName?: string;
  /** Environment variable prefix for workflow function names. Default: "WORKFLOW_" */
  envPrefix?: string;
  /** Path to the WASM plugin file. Auto-detected if not specified. */
  pluginPath?: string;
}

function resolvePluginWasm(customPath?: string): string {
  if (customPath) return customPath;

  // Try to find the WASM file relative to this package
  const candidates = [
    resolve(__dirname, "../wasm/plugin.wasm"),
    resolve(__dirname, "../../wasm/plugin.wasm"),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    "Could not locate plugin.wasm. Please specify pluginPath in options."
  );
}

function matchesPattern(filePath: string, patterns: string[]): boolean {
  // Simple glob matching — checks if the file path contains any pattern segment
  return patterns.some((pattern) => {
    // Convert simple glob to check
    const normalized = pattern.replace(/\*/g, "");
    return filePath.includes(normalized) || filePath.endsWith(normalized);
  });
}

export function durablePlugin(options: DurablePluginOptions = {}): {
  name: string;
  setup: (build: { onLoad: Function }) => void;
} {
  const {
    workflowPatterns = ["**/workflows/**", "**/*.workflow.*"],
    clientPatterns = ["**/handlers/**", "**/*.handler.*", "**/api/**"],
    packageName = "@cgalceran/aws-durable",
    envPrefix = "WORKFLOW_",
  } = options;

  const pluginWasm = resolvePluginWasm(options.pluginPath);

  return {
    name: "aws-durable",
    setup(build) {
      build.onLoad(
        { filter: /\.(ts|tsx|js|jsx)$/ },
        async (args: { path: string }) => {
          const filePath = args.path;
          const source = await fsp.readFile(filePath, "utf-8");

          // Determine transform mode based on file path
          let mode: "workflow" | "client" | null = null;

          if (matchesPattern(filePath, workflowPatterns)) {
            // Only transform if file actually has directives
            if (
              source.includes('"use workflow"') ||
              source.includes("'use workflow'") ||
              source.includes('"use step"') ||
              source.includes("'use step'")
            ) {
              mode = "workflow";
            }
          } else if (matchesPattern(filePath, clientPatterns)) {
            mode = "client";
          }

          if (!mode) {
            return undefined; // Let esbuild handle normally
          }

          const result = await transform(source, {
            filename: filePath,
            jsc: {
              parser: {
                syntax: "typescript",
                tsx: filePath.endsWith(".tsx"),
              },
              experimental: {
                plugins: [
                  [
                    pluginWasm,
                    {
                      mode,
                      packageName,
                      envPrefix,
                    },
                  ],
                ],
              },
              target: "es2022",
            },
          });

          return {
            contents: result.code,
            loader: "js" as const,
          };
        }
      );
    },
  };
}
