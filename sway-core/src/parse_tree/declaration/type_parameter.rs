use crate::{
    error::{err, ok},
    parse_tree::*,
    semantic_analysis::*,
    type_engine::*,
    CompileError, CompileResult,
};

use sway_types::{ident::Ident, span::Span, Spanned};

use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Eq)]
pub struct TypeParameter {
    pub(crate) type_id: TypeId,
    pub(crate) name_ident: Ident,
    pub(crate) trait_constraints: Vec<TraitConstraint>,
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl Hash for TypeParameter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        look_up_type_id(self.type_id).hash(state);
        self.name_ident.hash(state);
        self.trait_constraints.hash(state);
    }
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl PartialEq for TypeParameter {
    fn eq(&self, other: &Self) -> bool {
        look_up_type_id(self.type_id) == look_up_type_id(other.type_id)
            && self.name_ident == other.name_ident
            && self.trait_constraints == other.trait_constraints
    }
}

impl CopyTypes for TypeParameter {
    fn copy_types(&mut self, type_mapping: &TypeMapping) {
        self.type_id = match look_up_type_id(self.type_id).matches_type_parameter(type_mapping) {
            Some(matching_id) => insert_type(TypeInfo::Ref(matching_id, self.name_ident.span())),
            None => {
                let ty = TypeInfo::Ref(insert_type(look_up_type_id_raw(self.type_id)), self.span());
                insert_type(ty)
            }
        };
    }
}

impl Spanned for TypeParameter {
    fn span(&self) -> Span {
        self.name_ident.span()
    }
}

impl TypeParameter {
    pub(crate) fn type_check(&mut self, namespace: &mut Namespace) -> CompileResult<()> {
        let warnings = vec![];
        let mut errors = vec![];
        self.type_id = insert_type(TypeInfo::UnknownGeneric {
            name: self.name_ident.clone(),
        });
        match look_up_type_id(self.type_id) {
            TypeInfo::Custom {
                name,
                type_arguments,
            } => {
                if !type_arguments.is_empty() {
                    let type_arguments_span = type_arguments
                        .iter()
                        .map(|x| x.span.clone())
                        .reduce(Span::join)
                        .unwrap_or_else(|| name.span());
                    errors.push(CompileError::TypeArgumentsNotAllowedInTypeParameters(
                        type_arguments_span,
                    ));
                    err(warnings, errors)
                } else {
                    let type_parameter_decl = TypedDeclaration::GenericTypeInScope {
                        name: self.name_ident.clone(),
                        type_id: self.type_id,
                    };
                    namespace.insert_symbol(self.name_ident.clone(), type_parameter_decl);
                    ok((), warnings, errors)
                }
            }
            ty => {
                errors.push(CompileError::InvalidGenericTypeName {
                    ty: ty.to_string(),
                    span: self.span(),
                });
                err(warnings, errors)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct TraitConstraint {
    pub(crate) call_path: CallPath,
}
