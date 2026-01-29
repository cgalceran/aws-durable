import { signupWorkflow } from "./workflows/signup";
import { logger } from "shared-utils";

export async function handleRequest(req: Request) {
    logger.info("Starting signup");
    const result = await signupWorkflow({ email: req.body.email, name: req.body.name });
    return result;
}
