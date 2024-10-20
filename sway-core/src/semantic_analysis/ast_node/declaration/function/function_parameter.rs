use crate::{
    error::ok,
    semantic_analysis::{
        EnforceTypeArguments, IsConstant, TypedExpression, TypedExpressionVariant,
        TypedVariableDeclaration, VariableMutability,
    },
    type_engine::*,
    CompileResult, FunctionParameter, Ident, Namespace, TypedDeclaration,
};

use sway_types::{span::Span, Spanned};

#[derive(Debug, Clone, Eq)]
pub struct TypedFunctionParameter {
    pub name: Ident,
    pub type_id: TypeId,
    pub(crate) type_span: Span,
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl PartialEq for TypedFunctionParameter {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && look_up_type_id(self.type_id) == look_up_type_id(other.type_id)
    }
}

impl CopyTypes for TypedFunctionParameter {
    fn copy_types(&mut self, type_mapping: &TypeMapping) {
        self.type_id.update_type(type_mapping, &self.type_span);
    }
}

impl TypedFunctionParameter {
    pub(crate) fn type_check(
        parameter: FunctionParameter,
        namespace: &mut Namespace,
        self_type: TypeId,
    ) -> CompileResult<TypedFunctionParameter> {
        let mut warnings = vec![];
        let mut errors = vec![];
        let type_id = check!(
            namespace.resolve_type_with_self(
                look_up_type_id(parameter.type_id),
                self_type,
                &parameter.type_span,
                EnforceTypeArguments::Yes
            ),
            insert_type(TypeInfo::ErrorRecovery),
            warnings,
            errors,
        );
        namespace.insert_symbol(
            parameter.name.clone(),
            TypedDeclaration::VariableDeclaration(TypedVariableDeclaration {
                name: parameter.name.clone(),
                body: TypedExpression {
                    expression: TypedExpressionVariant::FunctionParameter,
                    return_type: type_id,
                    is_constant: IsConstant::No,
                    span: parameter.name.span(),
                },
                is_mutable: VariableMutability::Immutable,
                const_decl_origin: false,
                type_ascription: type_id,
            }),
        );
        let parameter = TypedFunctionParameter {
            name: parameter.name,
            type_id,
            type_span: parameter.type_span,
        };
        ok(parameter, warnings, errors)
    }
}
