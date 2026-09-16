use crate::ast::*;

type FunctionDef<'a> = (&'a str, &'a [String], &'a [Option<String>], &'a [Stmt]);

pub fn collect_function_defs<'a>(stmts: &'a [Stmt], defs: &mut Vec<FunctionDef<'a>>) {
    for stmt in stmts {
        match stmt {
            Stmt::Function {
                name,
                params,
                param_types,
                body,
                ..
            } => {
                defs.push((
                    name.as_str(),
                    params.as_slice(),
                    param_types.as_slice(),
                    body.as_slice(),
                ));
                collect_function_defs(body, defs);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_function_defs(then_block, defs);
                if let Some(eb) = else_block {
                    collect_function_defs(eb, defs);
                }
            }
            Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Repeat { body } => {
                collect_function_defs(body, defs);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_function_defs(try_block, defs);
                collect_function_defs(catch_block, defs);
                if let Some(finally_block) = finally_block {
                    collect_function_defs(finally_block, defs);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_function_defs(std::slice::from_ref(inner), defs);
            }
            _ => {}
        }
    }
}
