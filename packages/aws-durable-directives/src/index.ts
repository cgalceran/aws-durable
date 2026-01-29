/**
 * Type stubs for directive functions.
 * These are erased at compile time by the SWC plugin.
 * They exist so TypeScript understands the API and provides autocompletion.
 */

/** Duration specification for sleep() */
export interface Duration {
  seconds?: number;
  minutes?: number;
  hours?: number;
  days?: number;
}

/** Workflow descriptor produced by client mode transform */
export interface WorkflowDescriptor {
  __workflow: true;
  name: string;
  functionName: string;
}

/**
 * Invoke another Lambda function as a durable step.
 * Transformed by the SWC plugin into a ctx.step() with LambdaClient invocation.
 *
 * @param functionName - The Lambda function name or ARN
 * @param payload - The payload to send
 * @returns The parsed response from the invoked function
 */
export declare function invoke<T = unknown>(
  functionName: string,
  payload: unknown
): Promise<T>;

/**
 * Sleep for a specified duration.
 * Transformed by the SWC plugin into a ctx.wait() call.
 *
 * @param duration - How long to sleep
 */
export declare function sleep(duration: Duration): Promise<void>;

/**
 * Options for waitForCallback */
export interface WaitForCallbackOptions {
  timeout?: Duration;
}

/**
 * Pause execution and wait for an external callback.
 * Transformed by the SWC plugin into a ctx.waitForCallback() call.
 *
 * @param name - Unique name for this callback point
 * @param setup - Optional setup function called with the callback token
 * @param opts - Optional timeout configuration
 * @returns The data sent with the callback
 */
export declare function waitForCallback<T = unknown>(
  name: string,
  setup?: (token: string) => void | Promise<void>,
  opts?: WaitForCallbackOptions
): Promise<T>;
