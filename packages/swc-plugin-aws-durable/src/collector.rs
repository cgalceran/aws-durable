use std::collections::HashMap;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::Visit;

use crate::config::PluginConfig;
use crate::directive::{block_has_step_directive, block_has_workflow_directive};

/// Info about a function with `"use workflow"` directive.
#[derive(Debug, Clone)]
pub struct WorkflowFnInfo {
    pub name: String,
    pub is_exported: bool,
    pub is_default_export: bool,
    pub is_async: bool,
}

/// Info about a function with `"use step"` directive.
#[derive(Debug, Clone)]
pub struct StepFnInfo {
    pub name: String,
    pub body: BlockStmt,
}

/// Info about an import from a workflow file (for client mode).
#[derive(Debug, Clone)]
pub struct WorkflowImportInfo {
    pub local_name: String,
    pub imported_name: String,
    pub source: String,
}

/// Collected information from the first pass.
#[derive(Debug, Clone, Default)]
pub struct CollectedInfo {
    pub workflow_fns: Vec<WorkflowFnInfo>,
    pub step_fns: HashMap<String, StepFnInfo>,
    pub workflow_imports: Vec<WorkflowImportInfo>,
    pub has_invoke: bool,
    pub has_sleep: bool,
    pub has_wait_for_callback: bool,
    /// Names of functions with step directives, so we can remove them.
    pub step_fn_names: Vec<String>,
    /// Whether the module itself has a top-level "use workflow" directive.
    pub has_module_workflow_directive: bool,
}

/// Pass 1: Read-only visitor that collects workflow/step information.
pub struct Collector<'a> {
    pub config: &'a PluginConfig,
    pub info: CollectedInfo,
    /// Track which module items are exports to associate with fn declarations.
    current_export: bool,
    current_default_export: bool,
}

impl<'a> Collector<'a> {
    pub fn new(config: &'a PluginConfig) -> Self {
        Self {
            config,
            info: CollectedInfo::default(),
            current_export: false,
            current_default_export: false,
        }
    }
}

impl Visit for Collector<'_> {
    fn visit_module_items(&mut self, items: &[ModuleItem]) {
        // Check top-level for "use workflow" directive
        for item in items {
            if let ModuleItem::Stmt(stmt) = item {
                if crate::directive::is_use_workflow_directive(stmt) {
                    self.info.has_module_workflow_directive = true;
                }
            }
        }

        // Collect imports from workflow files
        for item in items {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
                let src = import.src.value.to_string_lossy().into_owned();
                // In client mode, any relative import could be a workflow file.
                // We'll collect them and let the transform decide.
                if src.starts_with("./") || src.starts_with("../") {
                    for spec in &import.specifiers {
                        match spec {
                            ImportSpecifier::Named(named) => {
                                let imported = named
                                    .imported
                                    .as_ref()
                                    .map(|i| match i {
                                        ModuleExportName::Ident(id) => id.sym.to_string(),
                                        ModuleExportName::Str(s) => s.value.to_string_lossy().into_owned(),
                                        _ => named.local.sym.to_string(),
                                    })
                                    .unwrap_or_else(|| named.local.sym.to_string());
                                self.info.workflow_imports.push(WorkflowImportInfo {
                                    local_name: named.local.sym.to_string(),
                                    imported_name: imported,
                                    source: src.clone(),
                                });
                            }
                            ImportSpecifier::Default(def) => {
                                self.info.workflow_imports.push(WorkflowImportInfo {
                                    local_name: def.local.sym.to_string(),
                                    imported_name: "default".to_string(),
                                    source: src.clone(),
                                });
                            }
                            ImportSpecifier::Namespace(ns) => {
                                self.info.workflow_imports.push(WorkflowImportInfo {
                                    local_name: ns.local.sym.to_string(),
                                    imported_name: "*".to_string(),
                                    source: src.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Visit each item normally
        for item in items {
            match item {
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
                    self.current_export = true;
                    self.current_default_export = false;
                    self.visit_export_decl(export_decl);
                    self.current_export = false;
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export_default)) => {
                    self.current_export = true;
                    self.current_default_export = true;
                    self.visit_export_default_decl(export_default);
                    self.current_export = false;
                    self.current_default_export = false;
                }
                _ => {
                    self.visit_module_item(item);
                }
            }
        }
    }

    fn visit_fn_decl(&mut self, fn_decl: &FnDecl) {
        let name = fn_decl.ident.sym.to_string();
        if let Some(body) = &fn_decl.function.body {
            if block_has_workflow_directive(body) {
                self.info.workflow_fns.push(WorkflowFnInfo {
                    name: name.clone(),
                    is_exported: self.current_export,
                    is_default_export: self.current_default_export,
                    is_async: fn_decl.function.is_async,
                });
            }
            if block_has_step_directive(body) {
                self.info.step_fn_names.push(name.clone());
                self.info.step_fns.insert(
                    name.clone(),
                    StepFnInfo {
                        name,
                        body: body.clone(),
                    },
                );
            }
            // Scan body for special calls
            self.scan_block_for_special_calls(body);
        }
    }

    fn visit_var_declarator(&mut self, var: &VarDeclarator) {
        // Handle: const myWorkflow = async (params) => { "use workflow"; ... }
        if let Some(init) = &var.init {
            let name = match &var.name {
                Pat::Ident(ident) => Some(ident.sym.to_string()),
                _ => None,
            };

            if let Some(name) = name {
                let (is_async, body) = extract_fn_body(init);
                if let Some(body) = body {
                    if block_has_workflow_directive(&body) {
                        self.info.workflow_fns.push(WorkflowFnInfo {
                            name: name.clone(),
                            is_exported: self.current_export,
                            is_default_export: self.current_default_export,
                            is_async,
                        });
                    }
                    if block_has_step_directive(&body) {
                        self.info.step_fn_names.push(name.clone());
                        self.info.step_fns.insert(
                            name.clone(),
                            StepFnInfo { name, body: body.clone() },
                        );
                    }
                    self.scan_block_for_special_calls(&body);
                }
            }
        }
    }

    fn visit_call_expr(&mut self, call: &CallExpr) {
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Ident(ident) = expr.as_ref() {
                match ident.sym.as_ref() {
                    "invoke" => self.info.has_invoke = true,
                    "sleep" => self.info.has_sleep = true,
                    "waitForCallback" => self.info.has_wait_for_callback = true,
                    _ => {}
                }
            }
        }
        // Continue visiting children
        for arg in &call.args {
            self.visit_expr(&arg.expr);
        }
    }
}

impl Collector<'_> {
    fn scan_block_for_special_calls(&mut self, block: &BlockStmt) {
        use swc_core::ecma::visit::VisitWith;
        // Use a mini-visitor to scan for special calls within the block
        struct CallScanner {
            has_invoke: bool,
            has_sleep: bool,
            has_wait_for_callback: bool,
        }
        impl Visit for CallScanner {
            fn visit_call_expr(&mut self, call: &CallExpr) {
                if let Callee::Expr(expr) = &call.callee {
                    if let Expr::Ident(ident) = expr.as_ref() {
                        match ident.sym.as_ref() {
                            "invoke" => self.has_invoke = true,
                            "sleep" => self.has_sleep = true,
                            "waitForCallback" => self.has_wait_for_callback = true,
                            _ => {}
                        }
                    }
                }
                // Continue scanning nested calls
                for arg in &call.args {
                    self.visit_expr(&arg.expr);
                }
            }
        }

        let mut scanner = CallScanner {
            has_invoke: false,
            has_sleep: false,
            has_wait_for_callback: false,
        };
        block.visit_with(&mut scanner);
        self.info.has_invoke |= scanner.has_invoke;
        self.info.has_sleep |= scanner.has_sleep;
        self.info.has_wait_for_callback |= scanner.has_wait_for_callback;
    }
}

/// Extract function body and async flag from an expression (arrow fn or fn expr).
fn extract_fn_body(expr: &Expr) -> (bool, Option<BlockStmt>) {
    match expr {
        Expr::Arrow(arrow) => {
            let body = match &*arrow.body {
                BlockStmtOrExpr::BlockStmt(block) => Some(block.clone()),
                _ => None,
            };
            (arrow.is_async, body)
        }
        Expr::Fn(fn_expr) => {
            let body = fn_expr.function.body.clone();
            (fn_expr.function.is_async, body)
        }
        _ => (false, None),
    }
}
