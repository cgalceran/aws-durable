# aws-durable

This is a Speedy Web Compiler SWC plugin that transforms simple JS functions into the new AWS Lambda durable workflows using directives. I wanted to use directives while working on workflows just like vercel created their WDK, the only difference is that vercel's backend is distributed by default so their rust transformation convert JS functions into endpoints. In this case is different, the execution happens within the lambda so it's simpler, except on the client.

NOTE= Still a experimental package.

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

## License

MIT
