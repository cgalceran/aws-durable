use crate::collector::Collector;
use crate::config::{PluginConfig, TransformMode};
use crate::transform::WorkflowTransformer;
use swc_core::ecma::ast::*;
use swc_core::ecma::parser::{EsSyntax, Syntax};
use swc_core::ecma::visit::{VisitMut, VisitWith};

fn parse_module(code: &str) -> Module {
    use swc_core::common::sync::Lrc;
    use swc_core::common::{FileName, SourceMap};
    use swc_core::ecma::parser;

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Custom("test.ts".into()).into(),
        code.to_string(),
    );

    let mut errors = vec![];
    let module = parser::parse_file_as_module(
        &fm,
        Syntax::Es(EsSyntax {
            decorators: true,
            ..Default::default()
        }),
        swc_core::ecma::ast::EsVersion::Es2022,
        None,
        &mut errors,
    )
    .expect("Failed to parse");

    module
}

fn transform_workflow(code: &str) -> Module {
    let mut module = parse_module(code);
    let config = PluginConfig {
        mode: TransformMode::Workflow,
        ..Default::default()
    };

    let mut collector = Collector::new(&config);
    module.visit_with(&mut collector);

    let mut transformer = WorkflowTransformer::new(config.clone(), collector.info);
    transformer.visit_mut_module(&mut module);

    module
}

fn has_import(module: &Module, source: &str) -> bool {
    module.body.iter().any(|item| {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
            import.src.value == source
        } else {
            false
        }
    })
}

fn has_export_named(module: &Module, name: &str) -> bool {
    module.body.iter().any(|item| {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                decl: Decl::Var(var_decl),
                ..
            })) => var_decl.decls.iter().any(|d| {
                if let Pat::Ident(ident) = &d.name {
                    ident.sym.as_ref() == name
                } else {
                    false
                }
            }),
            _ => false,
        }
    })
}

#[test]
fn test_basic_workflow_transform() {
    let input = r#"
export async function myWorkflow(input) {
    "use workflow";
    const result = await doSomething(input);
    return result;
}
"#;

    let module = transform_workflow(input);

    // Should have SDK import
    assert!(has_import(&module, "@bento/aws-durable"));

    // Should have exported handler
    assert!(has_export_named(&module, "myWorkflow"));

    // Should have __workflowMeta export
    assert!(has_export_named(&module, "__workflowMeta"));
}

#[test]
fn test_no_directive_passthrough() {
    let input = r#"
export function normalFunction(input) {
    return input + 1;
}
"#;

    let module = transform_workflow(input);

    // Should NOT have SDK import (no workflow detected)
    assert!(!has_import(&module, "@bento/aws-durable"));

    // Module should be unchanged (1 item)
    assert_eq!(module.body.len(), 1);
}

#[test]
fn test_step_function_collection() {
    let input = r#"
async function validateInput(data) {
    "use step";
    if (!data.email) throw new Error("Missing email");
    return data;
}

export async function signupWorkflow(input) {
    "use workflow";
    const validated = await validateInput(input);
    return validated;
}
"#;

    let module = transform_workflow(input);

    // The step function should be removed (inlined)
    let has_validate_fn = module.body.iter().any(|item| {
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) = item {
            fn_decl.ident.sym.as_ref() == "validateInput"
        } else {
            false
        }
    });
    assert!(!has_validate_fn, "Step function should be removed");

    // Should have SDK import
    assert!(has_import(&module, "@bento/aws-durable"));
}

#[test]
fn test_invoke_generates_lambda_import() {
    let input = r#"
export async function orchestrator(input) {
    "use workflow";
    const result = await invoke("otherFn", input);
    return result;
}
"#;

    let module = transform_workflow(input);

    // Should have Lambda SDK import
    assert!(has_import(&module, "@aws-sdk/client-lambda"));
}

#[test]
fn test_sleep_transform() {
    let input = r#"
export async function delayedWorkflow(input) {
    "use workflow";
    await sleep({ seconds: 60 });
    return "done";
}
"#;

    let module = transform_workflow(input);

    // Should have SDK import but not Lambda import
    assert!(has_import(&module, "@bento/aws-durable"));
    assert!(!has_import(&module, "@aws-sdk/client-lambda"));
}
