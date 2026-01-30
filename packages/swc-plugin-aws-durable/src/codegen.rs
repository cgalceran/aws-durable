use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::*;

/// Create: `import { withDurableExecution } from "@cgalceran/aws-durable"`
pub fn create_sdk_import(package_name: &str) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
            span: DUMMY_SP,
            local: ident("withDurableExecution"),
            imported: None,
            is_type_only: false,
        })],
        src: Box::new(str_lit(package_name)),
        type_only: false,
        with: None,
        phase: Default::default(),
    }))
}

/// Create: `import { LambdaClient, InvokeCommand } from "@aws-sdk/client-lambda"`
pub fn create_lambda_sdk_import() -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("LambdaClient"),
                imported: None,
                is_type_only: false,
            }),
            ImportSpecifier::Named(ImportNamedSpecifier {
                span: DUMMY_SP,
                local: ident("InvokeCommand"),
                imported: None,
                is_type_only: false,
            }),
        ],
        src: Box::new(str_lit("@aws-sdk/client-lambda")),
        type_only: false,
        with: None,
        phase: Default::default(),
    }))
}

/// Create: `export const handler = withDurableExecution(async (event, ctx) => { ...body })`
pub fn create_with_durable_execution_call(
    fn_name: &str,
    body_stmts: Vec<Stmt>,
    is_exported: bool,
    is_default: bool,
) -> ModuleItem {
    let arrow = Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        params: vec![
            Pat::Ident(BindingIdent {
                id: ident("event"),
                type_ann: None,
            }),
            Pat::Ident(BindingIdent {
                id: ident("ctx"),
                type_ann: None,
            }),
        ],
        body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: DUMMY_SP,
            stmts: body_stmts,
            ctxt: Default::default(),
        })),
        is_async: true,
        is_generator: false,
        type_params: None,
        return_type: None,
        ctxt: Default::default(),
    });

    let call = Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Ident(ident("withDurableExecution")))),
        args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(arrow),
        }],
        type_args: None,
        ctxt: Default::default(),
    });

    let decl = VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident(fn_name),
                type_ann: None,
            }),
            init: Some(Box::new(call)),
            definite: false,
        }],
        ctxt: Default::default(),
    };

    if is_exported || is_default {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            span: DUMMY_SP,
            decl: Decl::Var(Box::new(decl)),
        }))
    } else {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(decl))))
    }
}

/// Create: `ctx.step("name", async () => { ...body })`
pub fn create_ctx_step_call(step_name: &str, body_stmts: Vec<Stmt>) -> Expr {
    let arrow = Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        params: vec![],
        body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: DUMMY_SP,
            stmts: body_stmts,
            ctxt: Default::default(),
        })),
        is_async: true,
        is_generator: false,
        type_params: None,
        return_type: None,
        ctxt: Default::default(),
    });

    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(ident("ctx"))),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "step".into(),
            }),
        }))),
        args: vec![
            ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(str_lit(step_name)))),
            },
            ExprOrSpread {
                spread: None,
                expr: Box::new(arrow),
            },
        ],
        type_args: None,
        ctxt: Default::default(),
    })
}

/// Create: `ctx.step("invoke:fnName", async () => { ... Lambda invocation ... })`
pub fn create_invoke_step(fn_name_expr: &Expr, payload_expr: &Expr) -> Expr {
    // Build: new LambdaClient({})
    let lambda_client = Expr::New(NewExpr {
        span: DUMMY_SP,
        callee: Box::new(Expr::Ident(ident("LambdaClient"))),
        args: Some(vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![],
            })),
        }]),
        type_args: None,
        ctxt: Default::default(),
    });

    // Build: new InvokeCommand({ FunctionName: fnName, Payload: JSON.stringify(payload) })
    let json_stringify = Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(ident("JSON"))),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "stringify".into(),
            }),
        }))),
        args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(payload_expr.clone()),
        }],
        type_args: None,
        ctxt: Default::default(),
    });

    let invoke_command = Expr::New(NewExpr {
        span: DUMMY_SP,
        callee: Box::new(Expr::Ident(ident("InvokeCommand"))),
        args: Some(vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Ident(IdentName {
                            span: DUMMY_SP,
                            sym: "FunctionName".into(),
                        }),
                        value: Box::new(fn_name_expr.clone()),
                    }))),
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Ident(IdentName {
                            span: DUMMY_SP,
                            sym: "Payload".into(),
                        }),
                        value: Box::new(json_stringify),
                    }))),
                ],
            })),
        }]),
        type_args: None,
        ctxt: Default::default(),
    });

    // Build: const client = new LambdaClient({});
    let client_decl = Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident("client"),
                type_ann: None,
            }),
            init: Some(Box::new(lambda_client)),
            definite: false,
        }],
        ctxt: Default::default(),
    })));

    // Build: const response = await client.send(new InvokeCommand({...}))
    let send_call = Expr::Await(AwaitExpr {
        span: DUMMY_SP,
        arg: Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(ident("client"))),
                prop: MemberProp::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "send".into(),
                }),
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(invoke_command),
            }],
            type_args: None,
            ctxt: Default::default(),
        })),
    });

    let response_decl = Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident("response"),
                type_ann: None,
            }),
            init: Some(Box::new(send_call)),
            definite: false,
        }],
        ctxt: Default::default(),
    })));

    // Build: return JSON.parse(new TextDecoder().decode(response.Payload))
    let return_stmt = Stmt::Return(ReturnStmt {
        span: DUMMY_SP,
        arg: Some(Box::new(Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(Expr::Ident(ident("JSON"))),
                prop: MemberProp::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "parse".into(),
                }),
            }))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::New(NewExpr {
                            span: DUMMY_SP,
                            callee: Box::new(Expr::Ident(ident("TextDecoder"))),
                            args: Some(vec![]),
                            type_args: None,
                            ctxt: Default::default(),
                        })),
                        prop: MemberProp::Ident(IdentName {
                            span: DUMMY_SP,
                            sym: "decode".into(),
                        }),
                    }))),
                    args: vec![ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Member(MemberExpr {
                            span: DUMMY_SP,
                            obj: Box::new(Expr::Ident(ident("response"))),
                            prop: MemberProp::Ident(IdentName {
                                span: DUMMY_SP,
                                sym: "Payload".into(),
                            }),
                        })),
                    }],
                    type_args: None,
                    ctxt: Default::default(),
                })),
            }],
            type_args: None,
            ctxt: Default::default(),
        }))),
    });

    create_ctx_step_call(
        "invoke",
        vec![client_decl, response_decl, return_stmt],
    )
}

/// Create: `ctx.wait({ seconds: N })`
pub fn create_ctx_wait_call(duration_expr: &Expr) -> Expr {
    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(ident("ctx"))),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "wait".into(),
            }),
        }))),
        args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(duration_expr.clone()),
        }],
        type_args: None,
        ctxt: Default::default(),
    })
}

/// Create: `ctx.waitForCallback(name, setup, opts)`
pub fn create_ctx_wait_for_callback_call(args: &[ExprOrSpread]) -> Expr {
    Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(ident("ctx"))),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "waitForCallback".into(),
            }),
        }))),
        args: args.to_vec(),
        type_args: None,
        ctxt: Default::default(),
    })
}

/// Create: `export const __workflowMeta = { name: "fnName", steps: [...] }`
pub fn create_workflow_meta_export(workflow_name: &str, step_names: &[String]) -> ModuleItem {
    let steps_array = Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: step_names
            .iter()
            .map(|name| {
                Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(str_lit(name)))),
                })
            })
            .collect(),
    });

    let meta_obj = Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "name".into(),
                }),
                value: Box::new(Expr::Lit(Lit::Str(str_lit(workflow_name)))),
            }))),
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "steps".into(),
                }),
                value: Box::new(steps_array),
            }))),
        ],
    });

    let decl = VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident("__workflowMeta"),
                type_ann: None,
            }),
            init: Some(Box::new(meta_obj)),
            definite: false,
        }],
        ctxt: Default::default(),
    };

    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: Decl::Var(Box::new(decl)),
    }))
}

/// Create: `export const X = { __workflow: true, name: "X", functionName: process.env.WORKFLOW_X }`
pub fn create_workflow_descriptor(local_name: &str, env_prefix: &str) -> ModuleItem {
    let env_var_name = format!(
        "{}{}",
        env_prefix,
        local_name.to_uppercase()
    );

    // process.env.WORKFLOW_X
    let env_access = Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Member(MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(Expr::Ident(ident("process"))),
            prop: MemberProp::Ident(IdentName {
                span: DUMMY_SP,
                sym: "env".into(),
            }),
        })),
        prop: MemberProp::Ident(IdentName {
            span: DUMMY_SP,
            sym: env_var_name.into(),
        }),
    });

    let descriptor = Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "__workflow".into(),
                }),
                value: Box::new(Expr::Lit(Lit::Bool(Bool {
                    span: DUMMY_SP,
                    value: true,
                }))),
            }))),
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "name".into(),
                }),
                value: Box::new(Expr::Lit(Lit::Str(str_lit(local_name)))),
            }))),
            PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName {
                    span: DUMMY_SP,
                    sym: "functionName".into(),
                }),
                value: Box::new(env_access),
            }))),
        ],
    });

    let decl = VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Const,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident(local_name),
                type_ann: None,
            }),
            init: Some(Box::new(descriptor)),
            definite: false,
        }],
        ctxt: Default::default(),
    };

    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(decl))))
}

// ── Helpers ──────────────────────────────────────────────────────────

fn ident(name: &str) -> Ident {
    Ident {
        span: DUMMY_SP,
        sym: name.into(),
        ctxt: Default::default(),
        optional: false,
    }
}

fn str_lit(value: &str) -> Str {
    Str {
        span: DUMMY_SP,
        value: value.into(),
        raw: None,
    }
}
