use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::{self as ast, Expr, Operator};
use ruff_python_semantic::analyze::typing::{
    collect_annotated_elements, AnnotatedShorthandOperator,
};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::fix::edits::pad;
use crate::{Applicability, Edit, Fix, FixAvailability, Violation};

/// ## What it does
/// Check for type annotations that can be rewritten based on the `@` shorthand
/// syntax.
///
/// ## Why is this bad?
/// Annotated type metadata can be expressed using the `@` operator.
/// This syntax is more concise and readable than the previous
/// `typing.Annotated` syntax.
///
/// This rule is enabled when targeting Python 3.16 or later (see:
/// [`target-version`]).
///
/// Unlike other `pyupgrade` rules, this rule is not enabled for earlier Python
/// versions even if `from __future__ import annotations` is present. This is
/// because `Annotated` is frequently used for runtime type introspection (e.g.,
/// with Pydantic), and the `@` shorthand is only available at runtime starting
/// with Python 3.16.
///
/// ## Example
/// ```python
/// from typing import Annotated
///
/// foo: Annotated[int, "metadata"] = 1
/// ```
///
/// Use instead:
/// ```python
/// foo: int @ "metadata" = 1
/// ```
///
/// ## Fix safety
/// This rule's fix is marked as unsafe, as it may remove comments if they
/// are present within the type annotation being rewritten. It may also lead to
/// runtime errors in unusual and likely incorrect type annotations where the type
/// does not support the `@` operator.
///
/// ## Options
/// - `target-version`
///
/// [shorthand syntax]: https://github.com/till-varoquaux/peps/blob/feature/at-type-annot/peps/pep-9999.rst
#[derive(ViolationMetadata)]
#[violation_metadata(stable_since = "v0.1.0")]
pub(crate) struct NonAnnotatedShorthand;

impl Violation for NonAnnotatedShorthand {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Use `T @ M` for type annotations".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Convert to `T @ M`".to_string())
    }
}

/// UP051
pub(crate) fn non_annotated_shorthand(
    checker: &Checker,
    expr: &Expr,
    _slice: &Expr,
    _operator: AnnotatedShorthandOperator,
) {
    // If we're at the top-level of an `Annotated` call, collect all the types and metadata.
    // We only want to flag the outermost `Annotated` call if it can be fully flattened.
    if let Some(parent) = checker.semantic().current_expression_parent() {
        if let Expr::Subscript(ast::ExprSubscript { value, .. }) = parent {
            if checker.semantic().match_typing_expr(value, "Annotated") {
                return;
            }
        }
    }

    let mut elements = Vec::new();
    collect_annotated_elements(expr, checker.semantic(), &mut elements);

    // If the base type is a string literal (e.g., `Annotated["int", "m"]`), it's likely a forward
    // reference. In such cases, we avoid rewriting to `@` syntax to preserve the user's intent.
    if let Some(first) = elements.first() {
        if matches!(first, Expr::StringLiteral(_)) {
            return;
        }
    } else {
        return;
    }

    // `NamedTuple` is not a type; it's a type constructor. Using it in a type annotation doesn't
    // make much sense. Consistent with PEP 604 rules, we ignore it.
    if elements.iter().any(|elt| is_named_tuple(checker, elt)) {
        return;
    }

    // Avoid fixing forward references, types not in an annotation, and expressions that would
    // lead to invalid syntax.
    let fixable = (checker.semantic().in_annotation()
        || checker.semantic().in_deferred_type_definition())
        && !checker.semantic().in_complex_string_type_definition()
        && elements.iter().all(|elt| is_allowed_value(elt));

    let has_comments = checker.comment_ranges().intersects(expr.range());

    let applicability = if has_comments {
        Applicability::Unsafe
    } else {
        Applicability::Safe
    };

    let guard = checker.report_diagnostic_if_enabled(NonAnnotatedShorthand, expr.range());

    let Some(mut diagnostic) = guard else {
        return;
    };

    if fixable {
        if let Some((first, rest)) = elements.split_first() {
            let mut fix_expr = (*first).clone();

            // The `@` operator is left-associative, so we can chain multiple metadata
            // arguments by repeatedly applying the operator.
            // For example, `Annotated[T, M1, M2]` becomes `(T @ M1) @ M2`.
            for metadata in rest {
                fix_expr = Expr::BinOp(ast::ExprBinOp {
                    left: Box::new(fix_expr),
                    op: Operator::MatMult,
                    right: Box::new((*metadata).clone()),
                    range: expr.range(),
                    node_index: ast::AtomicNodeIndex::default(),
                });
            }

            diagnostic.set_fix(Fix::applicable_edit(
                Edit::range_replacement(
                    pad(
                        checker.generator().expr(&fix_expr),
                        expr.range(),
                        checker.locator(),
                    ),
                    expr.range(),
                ),
                applicability,
            ));
        }
    }
}

/// Returns `true` if the expression is valid for use in a binary operation shorthand.
fn is_allowed_value(expr: &Expr) -> bool {
    match expr {
        Expr::BoolOp(_)
        | Expr::BinOp(_)
        | Expr::UnaryOp(_)
        | Expr::If(_)
        | Expr::Dict(_)
        | Expr::Set(_)
        | Expr::ListComp(_)
        | Expr::SetComp(_)
        | Expr::DictComp(_)
        | Expr::Generator(_)
        | Expr::Compare(_)
        | Expr::Call(_)
        | Expr::FString(_)
        | Expr::TString(_)
        | Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_)
        | Expr::Attribute(_)
        | Expr::Subscript(_)
        | Expr::Name(_)
        | Expr::List(_) => true,
        Expr::Tuple(tuple) => tuple.iter().all(is_allowed_value),
        Expr::Named(_) => false,
        // Invalid in binary expressions.
        Expr::Await(_)
        | Expr::Lambda(_)
        | Expr::Yield(_)
        | Expr::YieldFrom(_)
        | Expr::Starred(_)
        | Expr::Slice(_)
        | Expr::IpyEscapeCommand(_) => false,
    }
}

/// Return `true` if this is a `typing.NamedTuple` annotation.
fn is_named_tuple(checker: &Checker, expr: &Expr) -> bool {
    checker.semantic().match_typing_expr(expr, "NamedTuple")
}
