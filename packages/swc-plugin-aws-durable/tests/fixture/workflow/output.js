import { withDurableExecution } from "@bento/aws-durable";
import { LambdaClient, InvokeCommand } from "@aws-sdk/client-lambda";
export const signupWorkflow = withDurableExecution(async (event, ctx)=>{
    const email = await ctx.step("validateEmail", async ()=>{
        if (!event.email.includes("@")) throw new Error("Invalid email");
        return event.email.toLowerCase();
    });
    await ctx.wait({
        seconds: 5
    });
    const result = await ctx.step("invoke", async ()=>{
        const client = new LambdaClient({});
        const response = await client.send(new InvokeCommand({
            FunctionName: "sendWelcomeEmail",
            Payload: JSON.stringify({
                email,
                name: event.name
            })
        }));
        return JSON.parse(new TextDecoder().decode(response.Payload));
    });
    return {
        success: true,
        email,
        result
    };
});
export const __workflowMeta = {
    name: "signupWorkflow",
    steps: [
        "validateEmail"
    ]
};
