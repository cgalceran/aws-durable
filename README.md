# aws-durable

An SWC compiler plugin that transforms simple JavaScript functions into AWS Lambda durable workflows using directives.

Write workflow logic as plain async functions with `"use workflow"` and `"use step"` directives. The compiler handles the rest — wrapping your code in durable execution, checkpointing steps, and wiring up Lambda invocations.

## Before & After

**What you write:**

```ts
async function validateEmail(email: string) {
  "use step";
  if (!email.includes("@")) throw new Error("Invalid email");
  return email.toLowerCase();
}

export async function signupWorkflow(input: { email: string; name: string }) {
  "use workflow";
  const email = await validateEmail(input.email);
  await sleep({ seconds: 5 });
  const result = await invoke("sendWelcomeEmail", { email, name: input.name });
  return { success: true, email, result };
}
```

**What the compiler produces:**

```js
import { withDurableExecution } from "@cgalceran/aws-durable";
import { LambdaClient, InvokeCommand } from "@aws-sdk/client-lambda";

export const signupWorkflow = withDurableExecution(async (event, ctx) => {
  const email = await ctx.step("validateEmail", async () => {
    if (!event.email.includes("@")) throw new Error("Invalid email");
    return event.email.toLowerCase();
  });
  await ctx.wait({ seconds: 5 });
  const result = await ctx.step("invoke", async () => {
    const client = new LambdaClient({});
    const response = await client.send(
      new InvokeCommand({
        FunctionName: "sendWelcomeEmail",
        Payload: JSON.stringify({ email, name: event.name }),
      })
    );
    return JSON.parse(new TextDecoder().decode(response.Payload));
  });
  return { success: true, email, result };
});

export const __workflowMeta = {
  name: "signupWorkflow",
  steps: ["validateEmail"],
};
```

Step functions are inlined, special calls (`invoke`, `sleep`, `waitForCallback`) are rewritten, and a metadata export is generated for tooling.

## How It Works

The plugin runs two passes over your code:

1. **Collect** (read-only) — Scans for `"use workflow"` and `"use step"` directives, catalogs step function bodies, and detects special calls like `invoke()`, `sleep()`, and `waitForCallback()`.

2. **Transform** (rewrite) — Wraps workflow functions in `withDurableExecution()`, inlines step function bodies into `ctx.step()` calls, and replaces special calls with their SDK equivalents.

### Two Modes

| Mode | What it does |
|------|-------------|
| **workflow** | Transforms source files containing directives into durable execution code |
| **client** | Replaces imports from workflow files with lightweight descriptors (`{ __workflow: true, name, functionName }`) so client code can reference workflows without importing their implementation |

## Directives

### `"use workflow"`

Place at the top of a function body to mark it as a durable workflow. The function gets wrapped in `withDurableExecution()` which provides step checkpointing and replay.

```ts
export async function myWorkflow(input) {
  "use workflow";
  // your workflow logic
}
```

### `"use step"`

Place at the top of a function body to mark it as a named step. The function is removed from the output and its body is inlined at every call site as a `ctx.step()` call.

```ts
async function processPayment(amount: number) {
  "use step";
  // this body gets inlined into ctx.step("processPayment", async () => { ... })
}
```

### Built-in Functions

| Function | Compiles to | Purpose |
|----------|------------|---------|
| `invoke(fnName, payload)` | `ctx.step("invoke", ...)` with `LambdaClient` | Invoke another Lambda as a durable step |
| `sleep(duration)` | `ctx.wait(duration)` | Pause workflow execution |
| `waitForCallback(name, setup, opts)` | `ctx.waitForCallback(...)` | Suspend until an external callback arrives |

## Setup

### With esbuild

```ts
import { durablePlugin } from "@cgalceran/aws-durable-directives/esbuild";

esbuild.build({
  entryPoints: ["src/index.ts"],
  plugins: [
    durablePlugin({
      workflowPatterns: ["**/workflows/**"],
      clientPatterns: ["**/handlers/**"],
    }),
  ],
});
```

### With SWC directly

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        [
          "@cgalceran/aws-durable-directives/plugin",
          {
            "mode": "workflow",
            "packageName": "@cgalceran/aws-durable",
            "envPrefix": "WORKFLOW_"
          }
        ]
      ]
    }
  }
}
```

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `mode` | `"workflow"` | `"workflow"` to transform directives, `"client"` to generate descriptors |
| `packageName` | `"@cgalceran/aws-durable"` | The runtime package import path |
| `envPrefix` | `"WORKFLOW_"` | Prefix for environment variables in client mode descriptors |

## Why Rust?

SWC plugins must be compiled to WebAssembly (WASM). SWC chose this architecture so plugins run in a sandboxed, portable environment at near-native speed without blocking the main thread or requiring native Node addons.

Rust is the natural choice for this because:

- **SWC itself is written in Rust.** The plugin API (`swc_core`) exposes Rust types for the AST. Writing the plugin in Rust means direct access to the same AST types SWC uses internally — no serialization boundary, no impedance mismatch.
- **Rust compiles to WASM natively.** The `wasm32-wasip1` target is a first-class compilation target in `rustc`. No extra toolchains or transpilation steps.
- **Performance.** AST traversal and transformation on every file in a build benefits from compiled-language speed. The two-pass architecture (collect then transform) runs in microseconds per file.

## Packages

| Package | Language | Description |
|---------|----------|-------------|
| `packages/swc-plugin-aws-durable` | Rust | SWC plugin compiled to WASM — the compiler |
| `packages/aws-durable-directives` | TypeScript | npm package with type stubs, runtime, and esbuild plugin |

## Development

```bash
# Install Rust + WASM target
rustup target add wasm32-wasip1

# Run plugin tests
cd packages/swc-plugin-aws-durable
cargo test

# Build WASM
cargo build --target wasm32-wasip1 --release

# Copy to npm package
cp target/wasm32-wasip1/release/swc_plugin_aws_durable.wasm \
   ../aws-durable-directives/wasm/plugin.wasm

# Build TypeScript
cd ../aws-durable-directives
pnpm build
```

## License

MIT
