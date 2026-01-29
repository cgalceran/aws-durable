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
