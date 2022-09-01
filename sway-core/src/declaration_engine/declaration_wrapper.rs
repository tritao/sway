use std::fmt;

use sway_types::{Ident, Span};

use crate::{
    semantic_analysis::{
        TypedEnumDeclaration, TypedImplTrait, TypedStorageDeclaration, TypedStructDeclaration,
        TypedTraitDeclaration, TypedTraitFn,
    },
    type_system::{CopyTypes, TypeMapping},
    CompileError, MonomorphizeHelper, TypeParameter, TypedFunctionDeclaration,
};

/// The [DeclarationWrapper] type is used in the [DeclarationEngine]
/// as a means of placing all declaration types into the same type.
#[derive(Clone, Debug)]
pub(crate) enum DeclarationWrapper {
    // no-op variant to fulfill the default trait
    Unknown,
    Enum(TypedEnumDeclaration),
    Function(TypedFunctionDeclaration),
    Trait(TypedTraitDeclaration),
    TraitFn(TypedTraitFn),
    TraitImpl(TypedImplTrait),
    Struct(TypedStructDeclaration),
    Storage(TypedStorageDeclaration),
}

impl Default for DeclarationWrapper {
    fn default() -> Self {
        DeclarationWrapper::Unknown
    }
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl PartialEq for DeclarationWrapper {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DeclarationWrapper::Unknown, DeclarationWrapper::Unknown) => true,
            (DeclarationWrapper::Function(l), DeclarationWrapper::Function(r)) => l == r,
            (DeclarationWrapper::Trait(l), DeclarationWrapper::Trait(r)) => l == r,
            (DeclarationWrapper::TraitFn(l), DeclarationWrapper::TraitFn(r)) => l == r,
            (DeclarationWrapper::TraitImpl(l), DeclarationWrapper::TraitImpl(r)) => l == r,
            (DeclarationWrapper::Struct(l), DeclarationWrapper::Struct(r)) => l == r,
            (DeclarationWrapper::Storage(l), DeclarationWrapper::Storage(r)) => l == r,
            _ => false,
        }
    }
}

impl fmt::Display for DeclarationWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "decl({})", self.friendly_name())
    }
}

impl CopyTypes for DeclarationWrapper {
    fn copy_types(&mut self, type_mapping: &TypeMapping) {
        match self {
            DeclarationWrapper::Unknown => {}
            DeclarationWrapper::Enum(decl) => decl.copy_types(type_mapping),
            DeclarationWrapper::Function(decl) => decl.copy_types(type_mapping),
            DeclarationWrapper::Trait(decl) => decl.copy_types(type_mapping),
            DeclarationWrapper::TraitFn(decl) => decl.copy_types(type_mapping),
            DeclarationWrapper::TraitImpl(decl) => decl.copy_types(type_mapping),
            DeclarationWrapper::Struct(decl) => decl.copy_types(type_mapping),
            DeclarationWrapper::Storage(_) => {}
        }
    }
}

impl MonomorphizeHelper for DeclarationWrapper {
    fn type_parameters(&self) -> &[TypeParameter] {
        match self {
            DeclarationWrapper::Trait(_)
            | DeclarationWrapper::TraitFn(_)
            | DeclarationWrapper::TraitImpl(_)
            | DeclarationWrapper::Storage(_)
            | DeclarationWrapper::Unknown => {
                panic!("declaration type does not support type parameters")
            }
            DeclarationWrapper::Enum(decl) => decl.type_parameters(),
            DeclarationWrapper::Function(decl) => decl.type_parameters(),
            DeclarationWrapper::Struct(decl) => decl.type_parameters(),
        }
    }

    fn name(&self) -> &Ident {
        match self {
            DeclarationWrapper::Storage(_) | DeclarationWrapper::Unknown => {
                panic!("declaration type is not named")
            }
            DeclarationWrapper::Enum(decl) => decl.name(),
            DeclarationWrapper::Function(decl) => decl.name(),
            DeclarationWrapper::Trait(decl) => &decl.name,
            DeclarationWrapper::TraitFn(decl) => &decl.name,
            DeclarationWrapper::TraitImpl(decl) => &decl.trait_name.suffix,
            DeclarationWrapper::Struct(decl) => decl.name(),
        }
    }

    fn to_wrapper(&self) -> DeclarationWrapper {
        panic!("not expected to be called for DeclarationWrapper")
    }
}

impl DeclarationWrapper {
    /// friendly name string used for error reporting.
    fn friendly_name(&self) -> &'static str {
        match self {
            DeclarationWrapper::Unknown => "unknown",
            DeclarationWrapper::Enum(_) => "enum",
            DeclarationWrapper::Function(_) => "function",
            DeclarationWrapper::Trait(_) => "trait",
            DeclarationWrapper::Struct(_) => "struct",
            DeclarationWrapper::TraitImpl(_) => "impl trait",
            DeclarationWrapper::TraitFn(_) => "trait function",
            DeclarationWrapper::Storage(_) => "storage",
        }
    }

    pub(super) fn expect_function(
        self,
        span: &Span,
    ) -> Result<TypedFunctionDeclaration, CompileError> {
        match self {
            DeclarationWrapper::Function(decl) => Ok(decl),
            DeclarationWrapper::Unknown => Err(CompileError::Internal(
                "did not expect to find unknown declaration",
                span.clone(),
            )),
            actually => Err(CompileError::DeclIsNotAFunction {
                actually: actually.friendly_name().to_string(),
                span: span.clone(),
            }),
        }
    }

    pub(super) fn expect_trait(self, span: &Span) -> Result<TypedTraitDeclaration, CompileError> {
        match self {
            DeclarationWrapper::Trait(decl) => Ok(decl),
            DeclarationWrapper::Unknown => Err(CompileError::Internal(
                "did not expect to find unknown declaration",
                span.clone(),
            )),
            actually => Err(CompileError::DeclIsNotATrait {
                actually: actually.friendly_name().to_string(),
                span: span.clone(),
            }),
        }
    }

    pub(super) fn expect_trait_fn(self, span: &Span) -> Result<TypedTraitFn, CompileError> {
        match self {
            DeclarationWrapper::TraitFn(decl) => Ok(decl),
            DeclarationWrapper::Unknown => Err(CompileError::Internal(
                "did not expect to find unknown declaration",
                span.clone(),
            )),
            actually => Err(CompileError::DeclIsNotATraitFn {
                actually: actually.friendly_name().to_string(),
                span: span.clone(),
            }),
        }
    }

    pub(super) fn expect_trait_impl(self, span: &Span) -> Result<TypedImplTrait, CompileError> {
        match self {
            DeclarationWrapper::TraitImpl(decl) => Ok(decl),
            DeclarationWrapper::Unknown => Err(CompileError::Internal(
                "did not expect to find unknown declaration",
                span.clone(),
            )),
            actually => Err(CompileError::DeclIsNotAnImplTrait {
                actually: actually.friendly_name().to_string(),
                span: span.clone(),
            }),
        }
    }

    pub(super) fn expect_struct(self, span: &Span) -> Result<TypedStructDeclaration, CompileError> {
        match self {
            DeclarationWrapper::Struct(decl) => Ok(decl),
            DeclarationWrapper::Unknown => Err(CompileError::Internal(
                "did not expect to find unknown declaration",
                span.clone(),
            )),
            actually => Err(CompileError::DeclIsNotAStruct {
                actually: actually.friendly_name().to_string(),
                span: span.clone(),
            }),
        }
    }

    pub(super) fn expect_storage(
        self,
        span: &Span,
    ) -> Result<TypedStorageDeclaration, CompileError> {
        match self {
            DeclarationWrapper::Storage(decl) => Ok(decl),
            DeclarationWrapper::Unknown => Err(CompileError::Internal(
                "did not expect to find unknown declaration",
                span.clone(),
            )),
            actually => Err(CompileError::DeclIsNotStorage {
                actually: actually.friendly_name().to_string(),
                span: span.clone(),
            }),
        }
    }
}
