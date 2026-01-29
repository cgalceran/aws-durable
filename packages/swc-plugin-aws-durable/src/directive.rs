use swc_core::ecma::ast::*;

/// Check if a statement is a `"use workflow"` directive.
pub fn is_use_workflow_directive(stmt: &Stmt) -> bool {
    is_directive(stmt, "use workflow")
}

/// Check if a statement is a `"use step"` directive.
pub fn is_use_step_directive(stmt: &Stmt) -> bool {
    is_directive(stmt, "use step")
}

/// Check if a block statement contains a `"use workflow"` directive.
pub fn block_has_workflow_directive(block: &BlockStmt) -> bool {
    block.stmts.iter().any(|s| is_use_workflow_directive(s))
}

/// Check if a block statement contains a `"use step"` directive.
pub fn block_has_step_directive(block: &BlockStmt) -> bool {
    block.stmts.iter().any(|s| is_use_step_directive(s))
}

fn is_directive(stmt: &Stmt, value: &str) -> bool {
    match stmt {
        Stmt::Expr(ExprStmt { expr, .. }) => match expr.as_ref() {
            Expr::Lit(Lit::Str(s)) => s.value == value,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::common::DUMMY_SP;

    fn make_directive(value: &str) -> Stmt {
        Stmt::Expr(ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: value.into(),
                raw: None,
            }))),
        })
    }

    #[test]
    fn detects_use_workflow() {
        let stmt = make_directive("use workflow");
        assert!(is_use_workflow_directive(&stmt));
        assert!(!is_use_step_directive(&stmt));
    }

    #[test]
    fn detects_use_step() {
        let stmt = make_directive("use step");
        assert!(is_use_step_directive(&stmt));
        assert!(!is_use_workflow_directive(&stmt));
    }

    #[test]
    fn ignores_other_strings() {
        let stmt = make_directive("use strict");
        assert!(!is_use_workflow_directive(&stmt));
        assert!(!is_use_step_directive(&stmt));
    }
}
