import { logger } from "shared-utils";
const signupWorkflow = {
    __workflow: true,
    name: "signupWorkflow",
    functionName: process.env.WORKFLOW_SIGNUPWORKFLOW
};
export async function handleRequest(req) {
    logger.info("Starting signup");
    const result = await signupWorkflow({
        email: req.body.email,
        name: req.body.name
    });
    return result;
}
