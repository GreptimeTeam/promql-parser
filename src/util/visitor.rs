// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::parser::{
    AggregateExpr, BinaryExpr, Expr, Extension, ParenExpr, SubqueryExpr, UnaryExpr,
};

/// Trait that implements the [Visitor pattern](https://en.wikipedia.org/wiki/Visitor_pattern)
/// for a depth first walk on [Expr] AST. [`pre_visit`](ExprVisitor::pre_visit) is called
/// before any children are visited, and then [`post_visit`](ExprVisitor::post_visit) is called
/// after all children have been visited. Only [`pre_visit`](ExprVisitor::pre_visit) is required.
pub trait ExprVisitor {
    type Error;

    /// Called before any children are visited. Return `Ok(false)` to cut short the recursion
    /// (skip traversing and return).
    fn pre_visit(&mut self, plan: &Expr) -> Result<bool, Self::Error>;

    /// Called after all children are visited. Return `Ok(false)` to cut short the recursion
    /// (skip traversing and return).
    fn post_visit(&mut self, _plan: &Expr) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Trait that implements the [Visitor pattern](https://en.wikipedia.org/wiki/Visitor_pattern)
/// for a depth first walk on [Expr] AST. [`pre_visit`](ExprVisitorMut::pre_visit) is called
/// before any children are visited, and then [`post_visit`](ExprVisitorMut::post_visit) is called
/// after all children have been visited. Only [`pre_visit`](ExprVisitorMut::pre_visit) is required.
pub trait ExprVisitorMut {
    type Error;

    /// Called before any children are visited. Return `Ok(false)` to cut short the recursion
    /// (skip traversing and return).
    fn pre_visit(&mut self, plan: &mut Expr) -> Result<bool, Self::Error>;

    /// Called after all children are visited. Return `Ok(false)` to cut short the recursion
    /// (skip traversing and return).
    fn post_visit(&mut self, _plan: &mut Expr) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// A util function that traverses an AST [Expr] in depth-first order. Returns
/// `Ok(true)` if all nodes were visited, and `Ok(false)` if any call to
/// [`pre_visit`](ExprVisitor::pre_visit) or [`post_visit`](ExprVisitor::post_visit)
/// returned `Ok(false)` and may have cut short the recursion.
pub fn walk_expr<V: ExprVisitor>(visitor: &mut V, expr: &Expr) -> Result<bool, V::Error> {
    if !visitor.pre_visit(expr)? {
        return Ok(false);
    }

    let recurse = match expr {
        Expr::Aggregate(AggregateExpr { expr, .. }) => walk_expr(visitor, expr)?,
        Expr::Unary(UnaryExpr { expr }) => walk_expr(visitor, expr)?,
        Expr::Binary(BinaryExpr { lhs, rhs, .. }) => {
            walk_expr(visitor, lhs)? && walk_expr(visitor, rhs)?
        }
        Expr::Paren(ParenExpr { expr }) => walk_expr(visitor, expr)?,
        Expr::Subquery(SubqueryExpr { expr, .. }) => walk_expr(visitor, expr)?,
        Expr::Extension(Extension { expr }) => {
            for child in expr.children() {
                if !walk_expr(visitor, child)? {
                    return Ok(false);
                }
            }
            true
        }
        Expr::Call(call) => {
            for func_argument_expr in &call.args.args {
                if !walk_expr(visitor, func_argument_expr)? {
                    return Ok(false);
                }
            }
            true
        }
        Expr::NumberLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::VectorSelector(_)
        | Expr::MatrixSelector(_) => true,
    };

    if !recurse {
        return Ok(false);
    }

    if !visitor.post_visit(expr)? {
        return Ok(false);
    }

    Ok(true)
}

/// A util function that traverses an AST [Expr] mutably in depth-first order.
/// Returns `Ok(true)` if all nodes were visited, and `Ok(false)` if any call to
/// [`pre_visit`](ExprVisitorMut::pre_visit) or [`post_visit`](ExprVisitorMut::post_visit)
/// returned `Ok(false)` and may have cut short the recursion.
pub fn walk_expr_mut<V: ExprVisitorMut>(
    visitor: &mut V,
    expr: &mut Expr,
) -> Result<bool, V::Error> {
    if !visitor.pre_visit(expr)? {
        return Ok(false);
    }

    let recurse = match expr {
        Expr::Aggregate(AggregateExpr { expr, .. }) => walk_expr_mut(visitor, expr)?,
        Expr::Unary(UnaryExpr { expr }) => walk_expr_mut(visitor, expr)?,
        Expr::Binary(BinaryExpr { lhs, rhs, .. }) => {
            walk_expr_mut(visitor, lhs)? && walk_expr_mut(visitor, rhs)?
        }
        Expr::Paren(ParenExpr { expr }) => walk_expr_mut(visitor, expr)?,
        Expr::Subquery(SubqueryExpr { expr, .. }) => walk_expr_mut(visitor, expr)?,
        Expr::Extension(Extension { expr }) => {
            let mut children = expr.children().to_vec();
            let mut recurse = true;
            for child in &mut children {
                if !walk_expr_mut(visitor, child)? {
                    recurse = false;
                    break;
                }
            }
            *expr = expr.with_new_children(children);
            recurse
        }
        Expr::Call(call) => {
            for func_argument_expr in &mut call.args.args {
                if !walk_expr_mut(visitor, func_argument_expr)? {
                    return Ok(false);
                }
            }
            true
        }
        Expr::NumberLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::VectorSelector(_)
        | Expr::MatrixSelector(_) => true,
    };

    if !recurse {
        return Ok(false);
    }

    if !visitor.post_visit(expr)? {
        return Ok(false);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::label::MatchOp;
    use crate::parser;
    use crate::parser::ast::ExtensionExpr;
    use crate::parser::value::ValueType;
    use crate::parser::VectorSelector;
    use std::sync::Arc;

    struct NamespaceVisitor {
        namespace: String,
    }

    fn vector_selector_includes_namespace(
        namespace: &str,
        vector_selector: &VectorSelector,
    ) -> bool {
        let mut includes_namespace = false;
        for filters in &vector_selector.matchers.matchers {
            if filters.name.eq("namespace")
                && filters.value.eq(namespace)
                && filters.op == MatchOp::Equal
            {
                includes_namespace = true;
                break;
            }
        }
        includes_namespace
    }

    impl ExprVisitor for NamespaceVisitor {
        type Error = &'static str;

        fn pre_visit(&mut self, expr: &Expr) -> Result<bool, Self::Error> {
            match expr {
                Expr::VectorSelector(vector_selector) => {
                    let included = vector_selector_includes_namespace(
                        self.namespace.as_str(),
                        vector_selector,
                    );
                    return Ok(included);
                }
                Expr::MatrixSelector(matrix_selector) => {
                    let included = vector_selector_includes_namespace(
                        self.namespace.as_str(),
                        &matrix_selector.vs,
                    );
                    return Ok(included);
                }
                Expr::NumberLiteral(_) | Expr::StringLiteral(_) => return Ok(false),
                _ => (),
            }
            Ok(true)
        }
    }

    #[test]
    fn test_check_for_namespace_basic_query() {
        let expr = "pg_stat_activity_count{namespace=\"sample\"}";
        let ast = parser::parse(expr).unwrap();
        let mut visitor = NamespaceVisitor {
            namespace: "sample".to_string(),
        };
        assert!(walk_expr(&mut visitor, &ast).unwrap());
    }

    #[test]
    fn test_check_for_namespace_label_present() {
        let expr = "(sum by (namespace) (max_over_time(pg_stat_activity_count{namespace=\"sample\"}[1h])))";
        let ast = parser::parse(expr).unwrap();
        let mut visitor = NamespaceVisitor {
            namespace: "sample".to_string(),
        };
        assert!(walk_expr(&mut visitor, &ast).unwrap());
    }

    #[test]
    fn test_check_for_namespace_label_wrong_namespace() {
        let expr = "(sum by (namespace) (max_over_time(pg_stat_activity_count{namespace=\"sample\"}[1h])))";
        let ast = parser::parse(expr).unwrap();
        let mut visitor = NamespaceVisitor {
            namespace: "foobar".to_string(),
        };
        assert!(!walk_expr(&mut visitor, &ast).unwrap());
    }

    #[test]
    fn test_check_for_namespace_label_missing_namespace() {
        let expr = "(sum by (namespace) (max_over_time(pg_stat_activity_count{}[1h])))";
        let ast = parser::parse(expr).unwrap();
        let mut visitor = NamespaceVisitor {
            namespace: "sample".to_string(),
        };
        assert!(!walk_expr(&mut visitor, &ast).unwrap());
    }

    #[test]
    fn test_literal_expr() {
        let mut visitor = NamespaceVisitor {
            namespace: "sample".to_string(),
        };

        let ast = parser::parse("1").unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse("1 + 1").unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse(r#""1""#).unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());
    }

    #[test]
    fn test_binary_expr() {
        let mut visitor = NamespaceVisitor {
            namespace: "sample".to_string(),
        };

        let ast = parser::parse(
            "pg_stat_activity_count{namespace=\"sample\"} + pg_stat_activity_count{}",
        )
        .unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse(
            "pg_stat_activity_count{} - pg_stat_activity_count{namespace=\"sample\"}",
        )
        .unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse("pg_stat_activity_count{} * pg_stat_activity_count{}").unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse("pg_stat_activity_count{namespace=\"sample\"} / 1").unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse("1 % pg_stat_activity_count{namespace=\"sample\"}").unwrap();
        assert!(!walk_expr(&mut visitor, &ast).unwrap());

        let ast = parser::parse(
            "pg_stat_activity_count{namespace=\"sample\"} ^ \
             pg_stat_activity_count{namespace=\"sample\"}",
        )
        .unwrap();
        assert!(walk_expr(&mut visitor, &ast).unwrap());
    }

    struct LabelInjectorVisitor {
        label_name: String,
        label_value: String,
        inject_once: bool,
    }

    impl ExprVisitorMut for LabelInjectorVisitor {
        type Error = &'static str;

        fn pre_visit(&mut self, expr: &mut Expr) -> Result<bool, Self::Error> {
            if let Expr::VectorSelector(vector_selector) = expr {
                vector_selector
                    .matchers
                    .matchers
                    .push(crate::label::Matcher {
                        op: MatchOp::Equal,
                        name: self.label_name.clone(),
                        value: self.label_value.clone(),
                    });

                if self.inject_once {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }

    #[test]
    fn test_inject_label_into_vector_selector() {
        let expr = "pg_stat_activity_count{}";
        let mut ast = parser::parse(expr).unwrap();

        let mut visitor = LabelInjectorVisitor {
            label_name: "namespace".to_string(),
            label_value: "injected".to_string(),
            inject_once: false,
        };

        assert!(walk_expr_mut(&mut visitor, &mut ast).unwrap());

        if let Expr::VectorSelector(vs) = &ast {
            assert_eq!(vs.matchers.matchers.len(), 1);
            assert_eq!(vs.matchers.matchers[0].name, "namespace");
            assert_eq!(vs.matchers.matchers[0].value, "injected");
            assert_eq!(vs.matchers.matchers[0].op, MatchOp::Equal);
        } else {
            panic!("expected VectorSelector");
        }
    }

    #[test]
    fn test_inject_label_into_nested_expr() {
        let expr = "sum(pg_stat_activity_count{})";
        let mut ast = parser::parse(expr).unwrap();

        let mut visitor = LabelInjectorVisitor {
            label_name: "env".to_string(),
            label_value: "prod".to_string(),
            inject_once: false,
        };

        assert!(walk_expr_mut(&mut visitor, &mut ast).unwrap());

        if let Expr::Aggregate(agg) = &ast {
            if let Expr::VectorSelector(vs) = &*agg.expr {
                assert_eq!(vs.matchers.matchers.len(), 1);
                assert_eq!(vs.matchers.matchers[0].name, "env");
                assert_eq!(vs.matchers.matchers[0].value, "prod");
            } else {
                panic!("expected VectorSelector inside Aggregate");
            }
        } else {
            panic!("expected Aggregate");
        }
    }

    #[test]
    fn test_inject_label_into_multiple_selectors() {
        let expr = "pg_stat_activity_count{} + pg_stat_activity_count{}";
        let mut ast = parser::parse(expr).unwrap();

        let mut visitor = LabelInjectorVisitor {
            label_name: "env".to_string(),
            label_value: "prod".to_string(),
            inject_once: false,
        };

        assert!(walk_expr_mut(&mut visitor, &mut ast).unwrap());

        if let Expr::Binary(binary) = &ast {
            if let Expr::VectorSelector(lhs_vs) = &*binary.lhs {
                assert_eq!(lhs_vs.matchers.matchers.len(), 1);
                assert_eq!(lhs_vs.matchers.matchers[0].name, "env");
                assert_eq!(lhs_vs.matchers.matchers[0].value, "prod");
            } else {
                panic!("expected LHS to be a VectorSelector");
            }

            if let Expr::VectorSelector(rhs_vs) = &*binary.rhs {
                assert_eq!(rhs_vs.matchers.matchers.len(), 1);
                assert_eq!(rhs_vs.matchers.matchers[0].name, "env");
                assert_eq!(rhs_vs.matchers.matchers[0].value, "prod");
            } else {
                panic!("expected RHS to be a VectorSelector");
            }
        } else {
            panic!("expected a Binary expression");
        }
    }

    #[derive(Debug)]
    struct DummyExtension {
        children: Vec<Expr>,
    }

    impl ExtensionExpr for DummyExtension {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn name(&self) -> &str {
            "dummy"
        }
        fn value_type(&self) -> ValueType {
            ValueType::Vector
        }
        fn children(&self) -> &[Expr] {
            &self.children
        }
        fn with_new_children(&self, children: Vec<Expr>) -> Arc<dyn ExtensionExpr> {
            Arc::new(DummyExtension { children })
        }
    }

    #[test]
    fn test_inject_label_into_extension() {
        let inner_expr = parser::parse("pg_stat_activity_count{}").unwrap();
        let dummy_ext = DummyExtension {
            children: vec![inner_expr],
        };

        let shared_arc = std::sync::Arc::new(dummy_ext);
        // Clone the Arc to simulate multiple references to the same extension expression.
        let _second_reference = std::sync::Arc::clone(&shared_arc);

        let mut ast = Expr::Extension(parser::Extension { expr: shared_arc });

        let mut visitor = LabelInjectorVisitor {
            label_name: "env".to_string(),
            label_value: "prod".to_string(),
            inject_once: false,
        };
        assert!(walk_expr_mut(&mut visitor, &mut ast).unwrap());

        // The extension's children should be traversed and mutated like any other expression.
        if let Expr::Extension(ext) = &ast {
            let children = ext.expr.children();
            assert_eq!(children.len(), 1);
            if let Expr::VectorSelector(vs) = &children[0] {
                assert_eq!(vs.matchers.matchers.len(), 1);
                assert_eq!(vs.matchers.matchers[0].name, "env");
                assert_eq!(vs.matchers.matchers[0].value, "prod");
            } else {
                panic!("expected inner expression to be a VectorSelector");
            }
        } else {
            panic!("expected Extension expression");
        }
    }

    #[test]
    fn test_extension_partial_mutation_on_short_circuit() {
        let child1 = parser::parse("metric_a{}").unwrap();
        let child2 = parser::parse("metric_b{}").unwrap();

        let dummy_ext = DummyExtension {
            children: vec![child1, child2],
        };

        let mut ast = Expr::Extension(parser::Extension {
            expr: std::sync::Arc::new(dummy_ext),
        });

        let mut visitor = LabelInjectorVisitor {
            label_name: "env".to_string(),
            label_value: "prod".to_string(),
            inject_once: true,
        };

        // The walker returns Ok(false) because it short-circuits after mutating the first child.
        assert_eq!(walk_expr_mut(&mut visitor, &mut ast), Ok(false));

        if let Expr::Extension(ext) = &ast {
            let children = ext.expr.children();
            assert_eq!(children.len(), 2);

            // The first child should have been mutated.
            if let Expr::VectorSelector(vs) = &children[0] {
                assert_eq!(vs.matchers.matchers.len(), 1);
                assert_eq!(vs.matchers.matchers[0].name, "env");
                assert_eq!(vs.matchers.matchers[0].value, "prod");
            } else {
                panic!("expected first child to be a VectorSelector");
            }

            // The second child remains untouched.
            if let Expr::VectorSelector(vs) = &children[1] {
                assert!(vs.matchers.matchers.is_empty());
            } else {
                panic!("expected second child to be a VectorSelector");
            }
        } else {
            panic!("expected Extension expression");
        }
    }
}
