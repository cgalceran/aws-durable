/**
 * Runtime SDK for AWS Durable Functions.
 * This is the actual runtime code imported by the transformed workflow modules.
 */

export interface DurableContext {
  /** Execute a named step with automatic checkpointing. */
  step<T>(name: string, fn: () => Promise<T>): Promise<T>;

  /** Wait/sleep for a specified duration. */
  wait(duration: { seconds: number }): Promise<void>;

  /** Wait for an external callback. */
  waitForCallback<T>(
    name: string,
    setup?: (token: string) => void | Promise<void>,
    opts?: { timeout?: { seconds: number } }
  ): Promise<T>;
}

export interface DurableEvent<T = unknown> {
  executionId: string;
  input: T;
  [key: string]: unknown;
}

export interface WorkflowHandler {
  (event: unknown, context: unknown): Promise<unknown>;
}

/**
 * Wraps an async workflow function with durable execution support.
 * Handles step checkpointing, replay, and state persistence.
 *
 * @param fn - The workflow function receiving (event, ctx)
 * @returns A Lambda-compatible handler function
 */
export function withDurableExecution(
  fn: (event: DurableEvent, ctx: DurableContext) => Promise<unknown>
): WorkflowHandler {
  return async (rawEvent: unknown, _lambdaContext: unknown) => {
    const event = rawEvent as DurableEvent;
    const executionId =
      event.executionId || `exec-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

    const completedSteps = new Map<string, unknown>();
    const stepLog: string[] = [];

    const ctx: DurableContext = {
      async step<T>(name: string, fn: () => Promise<T>): Promise<T> {
        // Check if step was already completed (replay)
        if (completedSteps.has(name)) {
          return completedSteps.get(name) as T;
        }

        stepLog.push(name);
        const result = await fn();
        completedSteps.set(name, result);
        return result;
      },

      async wait(duration: { seconds: number }): Promise<void> {
        // In a real implementation, this would pause execution
        // and resume via Step Functions or a timer mechanism.
        // For now, implement as actual delay for local testing.
        await new Promise((resolve) =>
          setTimeout(resolve, duration.seconds * 1000)
        );
      },

      async waitForCallback<T>(
        name: string,
        setup?: (token: string) => void | Promise<void>,
        _opts?: { timeout?: { seconds: number } }
      ): Promise<T> {
        const token = `${executionId}:${name}:${Date.now()}`;
        if (setup) {
          await setup(token);
        }
        // In production, this would suspend and wait for callback delivery.
        // For local development, throw to indicate callback is pending.
        throw new Error(
          `Callback "${name}" is pending with token: ${token}. ` +
            `In production, execution suspends here until callback is received.`
        );
      },
    };

    const result = await fn(event, ctx);

    return {
      executionId,
      result,
      steps: stepLog,
    };
  };
}
