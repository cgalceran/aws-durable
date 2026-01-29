use swc_core::ecma::ast::*;
use swc_core::ecma::visit::VisitMut;

use crate::codegen;
use crate::collector::CollectedInfo;
use crate::config::{PluginConfig, TransformMode};
use crate::directive::{is_use_step_directive, is_use_workflow_directive};

/// Pass 2: Mutable visitor that transforms the AST.
pub struct WorkflowTransformer {
    pub config: PluginConfig,
    pub info: CollectedInfo,
    /// Whether we're currently inside a workflow function body.
    inside_workflow: bool,
}

impl WorkflowTransformer {
    pub fn new(config: PluginConfig, info: CollectedInfo) -> Self {
        Self {
            config,
            info,
            inside_workflow: false,
        }
    }

    fn is_step_fn_call(&self, callee: &Callee) -> Option<String> {
        if let Callee::Expr(expr) = callee {
            if let Expr::Ident(ident) = expr.as_ref() {
                let name = ident.sym.to_string();
                if self.info.step_fns.contains_key(&name) {
                    return Some(name);
                }
            }
        }
        None
    }

    fn is_special_call(&self, callee: &Callee) -> Option<String> {
        if let Callee::Expr(expr) = callee {
            if let Expr::Ident(ident) = expr.as_ref() {
                let name = ident.sym.as_ref();
                if matches!(name, "invoke" | "sleep" | "waitForCallback") {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    fn is_step_fn_name(&self, name: &str) -> bool {
        self.info.step_fn_names.iter().any(|n| n == name)
    }

    fn find_workflow_fn(&self, name: &str) -> Option<&crate::collector::WorkflowFnInfo> {
        self.info.workflow_fns.iter().find(|w| w.name == name)
    }
}

impl VisitMut for WorkflowTransformer {
    fn visit_mut_module(&mut self, module: &mut Module) {
        match self.config.mode {
            TransformMode::Workflow => self.transform_workflow_module(module),
            TransformMode::Client => self.transform_client_module(module),
        }
    }
}

impl WorkflowTransformer {
    // ── Workflow Mode ───────────────────────────────────────────

    fn transform_workflow_module(&mut self, module: &mut Module) {
        // No workflow directives found → skip entirely
        if self.info.workflow_fns.is_empty() && !self.info.has_module_workflow_directive {
            return;
        }

        let mut new_items: Vec<ModuleItem> = Vec::new();

        // 1. Prepend SDK imports
        new_items.push(codegen::create_sdk_import(&self.config.package_name));
        if self.info.has_invoke {
            new_items.push(codegen::create_lambda_sdk_import());
        }

        // 2. Process each module item
        for item in module.body.drain(..) {
            match &item {
                // Remove top-level "use workflow" directives
                ModuleItem::Stmt(stmt) if is_use_workflow_directive(stmt) => continue,

                // Handle function declarations (both step and workflow)
                ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                    let name = fn_decl.ident.sym.to_string();

                    // Remove step function declarations (they get inlined)
                    if self.is_step_fn_name(&name) {
                        continue;
                    }

                    // Transform workflow function declarations
                    if let Some(wf_info) = self.find_workflow_fn(&name).cloned() {
                        if let Some(body) = &fn_decl.function.body {
                            let stmts = self.transform_workflow_body(&body.stmts);
                            new_items.push(codegen::create_with_durable_execution_call(
                                &wf_info.name,
                                stmts,
                                false,
                                false,
                            ));
                        }
                        continue;
                    }

                    // Keep as-is
                    new_items.push(item);
                }

                // Handle var declarations (both step and workflow)
                ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                    // Check if this is a step fn var decl to remove
                    let is_step = var_decl.decls.iter().any(|d| {
                        if let Pat::Ident(ident) = &d.name {
                            self.is_step_fn_name(&ident.sym.to_string())
                        } else {
                            false
                        }
                    });
                    if is_step {
                        continue;
                    }

                    // Check if this is a workflow var decl to transform
                    let mut handled = false;
                    for d in &var_decl.decls {
                        if let Pat::Ident(ident) = &d.name {
                            let name = ident.sym.to_string();
                            if let Some(wf_info) = self.find_workflow_fn(&name).cloned() {
                                if let Some(init) = &d.init {
                                    if let Some(body) = extract_arrow_body(init) {
                                        let stmts = self.transform_workflow_body(&body.stmts);
                                        new_items.push(
                                            codegen::create_with_durable_execution_call(
                                                &wf_info.name,
                                                stmts,
                                                false,
                                                false,
                                            ),
                                        );
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }
                    if !handled {
                        new_items.push(item);
                    }
                }

                // Transform exported function declarations
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                    decl: Decl::Fn(fn_decl),
                    ..
                })) => {
                    let name = fn_decl.ident.sym.to_string();
                    if let Some(wf_info) = self.find_workflow_fn(&name).cloned() {
                        if let Some(body) = &fn_decl.function.body {
                            let stmts = self.transform_workflow_body(&body.stmts);
                            new_items.push(codegen::create_with_durable_execution_call(
                                &wf_info.name,
                                stmts,
                                true,
                                wf_info.is_default_export,
                            ));
                        }
                    } else {
                        new_items.push(item);
                    }
                }

                // Transform exported var decl workflow functions (arrow fns)
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                    decl: Decl::Var(var_decl),
                    ..
                })) => {
                    let mut handled = false;
                    for d in &var_decl.decls {
                        if let Pat::Ident(ident) = &d.name {
                            let name = ident.sym.to_string();
                            if let Some(wf_info) = self.find_workflow_fn(&name).cloned() {
                                if let Some(init) = &d.init {
                                    if let Some(body) = extract_arrow_body(init) {
                                        let stmts = self.transform_workflow_body(&body.stmts);
                                        new_items.push(
                                            codegen::create_with_durable_execution_call(
                                                &wf_info.name,
                                                stmts,
                                                true,
                                                wf_info.is_default_export,
                                            ),
                                        );
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }
                    if !handled {
                        new_items.push(item);
                    }
                }

                // Keep everything else
                _ => {
                    new_items.push(item);
                }
            }
        }

        // 3. Append __workflowMeta export
        if let Some(wf) = self.info.workflow_fns.first() {
            let step_names: Vec<String> = self.info.step_fns.keys().cloned().collect();
            new_items.push(codegen::create_workflow_meta_export(&wf.name, &step_names));
        }

        module.body = new_items;
    }

    fn transform_workflow_body(&mut self, stmts: &[Stmt]) -> Vec<Stmt> {
        self.inside_workflow = true;
        let result: Vec<Stmt> = stmts
            .iter()
            .filter(|s| !is_use_workflow_directive(s) && !is_use_step_directive(s))
            .map(|s| self.transform_stmt(s))
            .collect();
        self.inside_workflow = false;
        result
    }

    fn transform_stmt(&mut self, stmt: &Stmt) -> Stmt {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                let new_expr = self.transform_expr(&expr_stmt.expr);
                Stmt::Expr(ExprStmt {
                    span: expr_stmt.span,
                    expr: Box::new(new_expr),
                })
            }
            Stmt::Decl(Decl::Var(var_decl)) => {
                let new_decls: Vec<VarDeclarator> = var_decl
                    .decls
                    .iter()
                    .map(|d| VarDeclarator {
                        span: d.span,
                        name: d.name.clone(),
                        init: d.init.as_ref().map(|e| Box::new(self.transform_expr(e))),
                        definite: d.definite,
                    })
                    .collect();
                Stmt::Decl(Decl::Var(Box::new(VarDecl {
                    span: var_decl.span,
                    kind: var_decl.kind,
                    declare: var_decl.declare,
                    decls: new_decls,
                    ctxt: var_decl.ctxt,
                })))
            }
            Stmt::Return(ret) => Stmt::Return(ReturnStmt {
                span: ret.span,
                arg: ret
                    .arg
                    .as_ref()
                    .map(|e| Box::new(self.transform_expr(e))),
            }),
            // For other statement types, clone as-is
            other => other.clone(),
        }
    }

    fn transform_expr(&mut self, expr: &Expr) -> Expr {
        match expr {
            Expr::Call(call) => {
                // Check for step function call
                if let Some(step_name) = self.is_step_fn_call(&call.callee) {
                    if let Some(step_info) = self.info.step_fns.get(&step_name).cloned() {
                        let body_stmts: Vec<Stmt> = step_info
                            .body
                            .stmts
                            .iter()
                            .filter(|s| !is_use_step_directive(s))
                            .cloned()
                            .collect();
                        return codegen::create_ctx_step_call(&step_name, body_stmts);
                    }
                }

                // Check for special calls
                if let Some(special_name) = self.is_special_call(&call.callee) {
                    match special_name.as_str() {
                        "invoke" => {
                            if call.args.len() >= 2 {
                                return codegen::create_invoke_step(
                                    &call.args[0].expr,
                                    &call.args[1].expr,
                                );
                            }
                        }
                        "sleep" => {
                            if let Some(arg) = call.args.first() {
                                return codegen::create_ctx_wait_call(&arg.expr);
                            }
                        }
                        "waitForCallback" => {
                            return codegen::create_ctx_wait_for_callback_call(&call.args);
                        }
                        _ => {}
                    }
                }

                // Recurse into call arguments
                let new_args: Vec<ExprOrSpread> = call
                    .args
                    .iter()
                    .map(|arg| ExprOrSpread {
                        spread: arg.spread,
                        expr: Box::new(self.transform_expr(&arg.expr)),
                    })
                    .collect();

                Expr::Call(CallExpr {
                    span: call.span,
                    callee: call.callee.clone(),
                    args: new_args,
                    type_args: call.type_args.clone(),
                    ctxt: call.ctxt,
                })
            }
            Expr::Await(await_expr) => {
                let inner = self.transform_expr(&await_expr.arg);
                Expr::Await(AwaitExpr {
                    span: await_expr.span,
                    arg: Box::new(inner),
                })
            }
            Expr::Assign(assign) => Expr::Assign(AssignExpr {
                span: assign.span,
                op: assign.op,
                left: assign.left.clone(),
                right: Box::new(self.transform_expr(&assign.right)),
            }),
            // Pass through everything else
            other => other.clone(),
        }
    }

    // ── Client Mode ────────────────────────────────────────────

    fn transform_client_module(&mut self, module: &mut Module) {
        if self.info.workflow_imports.is_empty() {
            return;
        }

        let mut new_items: Vec<ModuleItem> = Vec::new();
        let mut descriptors_to_add: Vec<ModuleItem> = Vec::new();

        // Collect sources that have workflow imports
        let workflow_sources: Vec<String> = self
            .info
            .workflow_imports
            .iter()
            .map(|i| i.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for item in module.body.drain(..) {
            match &item {
                ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                    let src = import.src.value.to_string_lossy().into_owned();
                    if workflow_sources.contains(&src) {
                        // Replace this import with descriptors
                        for spec in &import.specifiers {
                            let local_name = match spec {
                                ImportSpecifier::Named(n) => n.local.sym.to_string(),
                                ImportSpecifier::Default(d) => d.local.sym.to_string(),
                                ImportSpecifier::Namespace(ns) => ns.local.sym.to_string(),
                                _ => continue,
                            };
                            descriptors_to_add.push(codegen::create_workflow_descriptor(
                                &local_name,
                                &self.config.env_prefix,
                            ));
                        }
                    } else {
                        new_items.push(item);
                    }
                }
                _ => {
                    new_items.push(item);
                }
            }
        }

        // Insert descriptors at the position where imports were
        let mut final_items = Vec::new();
        // Find first non-import item position
        let first_non_import = new_items
            .iter()
            .position(|item| !matches!(item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))))
            .unwrap_or(new_items.len());

        final_items.extend(new_items[..first_non_import].iter().cloned());
        final_items.extend(descriptors_to_add);
        final_items.extend(new_items[first_non_import..].iter().cloned());

        module.body = final_items;
    }
}

fn extract_arrow_body(expr: &Expr) -> Option<BlockStmt> {
    match expr {
        Expr::Arrow(arrow) => match &*arrow.body {
            BlockStmtOrExpr::BlockStmt(block) => Some(block.clone()),
            _ => None,
        },
        Expr::Fn(fn_expr) => fn_expr.function.body.clone(),
        _ => None,
    }
}
