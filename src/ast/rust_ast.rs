use syn::visit::Visit;

/// The type of call site found in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallType {
    /// receiver.method(args) — e.g. `vec.push(1)`
    MethodCall,
    /// Type::function(args) — e.g. `Vec::new()`
    AssociatedCall,
    /// function(args) — e.g. `println!("hi")` or `drop(x)`
    FreeCall,
}

/// A single call site extracted from source code via AST parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    pub call_type: CallType,
    /// For MethodCall: the receiver expression text (e.g. "vec", "self.items")
    /// For AssociatedCall: the type/path (e.g. "Vec", "HashMap")
    /// For FreeCall: empty string
    pub receiver: String,
    /// The method or function name being called
    pub method_name: String,
    /// 1-based line number in source
    pub line_number: usize,
    /// Number of arguments passed (excludes receiver for method calls)
    pub arg_count: usize,
}

/// Errors that can occur during AST extraction.
#[derive(Debug, thiserror::Error)]
pub enum AstError {
    #[error("failed to parse Rust source: {0}")]
    ParseError(String),
}

/// Extract all call sites from Rust source code using syn AST parsing.
///
/// Returns a list of `CallSite` structs representing every function/method
/// call found in the source. Falls back gracefully on parse errors by
/// returning the error (callers can then use regex fallback).
pub fn extract_calls_from_source(source: &str) -> Result<Vec<CallSite>, AstError> {
    let file = syn::parse_file(source).map_err(|e| AstError::ParseError(e.to_string()))?;
    let mut visitor = CallVisitor {
        calls: Vec::new(),
    };
    visitor.visit_file(&file);
    Ok(visitor.calls)
}

/// Count the arguments in a call expression from AST.
/// This is a utility for counting args given a `syn::ExprCall` or `syn::ExprMethodCall`.
pub fn count_call_args_ast(args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>) -> usize {
    args.len()
}

/// Extract the chained receiver for a method call expression.
/// Walks through nested method calls to find the root receiver.
/// E.g. for `foo.bar().baz()`, calling on `.baz()` returns "foo".
pub fn extract_chained_receiver(expr: &syn::Expr) -> String {
    match expr {
        syn::Expr::Path(ep) => path_to_string(&ep.path),
        syn::Expr::Field(ef) => {
            let base = extract_chained_receiver(&ef.base);
            let member = match &ef.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(idx) => idx.index.to_string(),
            };
            if base.is_empty() {
                member
            } else {
                format!("{}.{}", base, member)
            }
        }
        syn::Expr::MethodCall(mc) => extract_chained_receiver(&mc.receiver),
        syn::Expr::Call(c) => extract_chained_receiver(&c.func),
        syn::Expr::Reference(r) => extract_chained_receiver(&r.expr),
        syn::Expr::Paren(p) => extract_chained_receiver(&p.expr),
        syn::Expr::Try(t) => extract_chained_receiver(&t.expr),
        syn::Expr::Await(a) => extract_chained_receiver(&a.base),
        _ => String::new(),
    }
}

struct CallVisitor {
    calls: Vec<CallSite>,
}

impl CallVisitor {
    fn line_number_for_span(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }
}

impl<'a> Visit<'a> for CallVisitor {
    fn visit_expr_method_call(&mut self, node: &'a syn::ExprMethodCall) {
        let line_number = self.line_number_for_span(node.method.span());
        let method_name = node.method.to_string();
        let receiver = extract_chained_receiver(&node.receiver);
        let arg_count = count_call_args_ast(&node.args);

        self.calls.push(CallSite {
            call_type: CallType::MethodCall,
            receiver,
            method_name,
            line_number,
            arg_count,
        });

        // Continue visiting nested expressions
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'a syn::ExprCall) {
        match &*node.func {
            syn::Expr::Path(ep) => {
                let segments: Vec<_> = ep.path.segments.iter().collect();
                let arg_count = count_call_args_ast(&node.args);

                if segments.len() >= 2 {
                    // Type::method() or module::function()
                    let type_name = segments[..segments.len() - 1]
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    let fn_name = segments.last().unwrap().ident.to_string();
                    let line_number = self.line_number_for_span(
                        segments.last().unwrap().ident.span(),
                    );

                    self.calls.push(CallSite {
                        call_type: CallType::AssociatedCall,
                        receiver: type_name,
                        method_name: fn_name,
                        line_number,
                        arg_count,
                    });
                } else if segments.len() == 1 {
                    // free function call
                    let fn_name = segments[0].ident.to_string();
                    let line_number = self.line_number_for_span(segments[0].ident.span());

                    self.calls.push(CallSite {
                        call_type: CallType::FreeCall,
                        receiver: String::new(),
                        method_name: fn_name,
                        line_number,
                        arg_count,
                    });
                }
            }
            _ => {
                // Complex call expression (closures, etc.) — skip
            }
        }

        // Continue visiting nested expressions
        syn::visit::visit_expr_call(self, node);
    }
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}
