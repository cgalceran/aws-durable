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

fn transform_client(code: &str) -> Module {
    let mut module = parse_module(code);
    let config = PluginConfig {
        mode: TransformMode::Client,
        ..Default::default()
    };

    let mut collector = Collector::new(&config);
    module.visit_with(&mut collector);

    let mut transformer = WorkflowTransformer::new(config.clone(), collector.info);
    transformer.visit_mut_module(&mut module);

    module
}

#[test]
fn test_client_mode_replaces_imports() {
    let input = r#"
import { myWorkflow } from "./workflows/my-workflow";
import { someUtil } from "some-package";

async function main() {
    await myWorkflow({ data: "test" });
}
"#;

    let module = transform_client(input);

    // The workflow import should be removed
    let has_workflow_import = module.body.iter().any(|item| {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
            import.src.value == "./workflows/my-workflow"
        } else {
            false
        }
    });
    assert!(
        !has_workflow_import,
        "Workflow import should be replaced with descriptor"
    );

    // Non-workflow imports should remain
    let has_util_import = module.body.iter().any(|item| {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
            import.src.value == "some-package"
        } else {
            false
        }
    });
    assert!(has_util_import, "Non-workflow imports should remain");

    // Should have a descriptor const
    let has_descriptor = module.body.iter().any(|item| {
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item {
            var_decl.decls.iter().any(|d| {
                if let Pat::Ident(ident) = &d.name {
                    ident.sym.as_ref() == "myWorkflow"
                } else {
                    false
                }
            })
        } else {
            false
        }
    });
    assert!(has_descriptor, "Should have workflow descriptor");
}

#[test]
fn test_client_mode_preserves_non_relative_imports() {
    let input = r#"
import { something } from "external-package";

export function handler() {
    return something();
}
"#;

    let module = transform_client(input);

    // Everything should be unchanged
    assert_eq!(module.body.len(), 2);
}

#[test]
fn test_client_mode_multiple_imports() {
    let input = r#"
import { workflowA } from "./workflows/a";
import { workflowB } from "./workflows/b";

async function run() {
    await workflowA();
    await workflowB();
}
"#;

    let module = transform_client(input);

    // Both workflow imports should be replaced
    let import_count = module
        .body
        .iter()
        .filter(|item| matches!(item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))))
        .count();
    assert_eq!(import_count, 0, "All relative imports should be replaced");

    // Should have 2 descriptor consts + the function
    let var_count = module
        .body
        .iter()
        .filter(|item| matches!(item, ModuleItem::Stmt(Stmt::Decl(Decl::Var(_)))))
        .count();
    assert_eq!(var_count, 2, "Should have 2 workflow descriptors");
}
