use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};
use std::{collections::HashMap, fmt};
use sway_error::error::CompileError;
use sway_types::{Ident, Span, Spanned};

use crate::{concurrent_slab::ConcurrentSlab, language::ty, TypeParameter};

use super::{declaration_id::DeclarationId, declaration_wrapper::DeclarationWrapper};

lazy_static! {
    static ref DECLARATION_ENGINE: DeclarationEngine = DeclarationEngine::default();
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct DeclarationSignature {
    path: Arc<std::path::PathBuf>,
    name: Ident,
    type_parameters: Vec<TypeParameter>,
}

#[derive(PartialEq)]
pub enum CacheLookup {
    Enable,
    Disable,
}

/// Used inside of type inference to store declarations.
#[derive(Default)]
pub struct DeclarationEngine {
    slab: ConcurrentSlab<DeclarationWrapper>,
    decls: RwLock<HashMap<DeclarationSignature, DeclarationId>>,
}

impl fmt::Display for DeclarationEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeclarationEngine {{\n{}\n}}", self.slab)
    }
}

impl DeclarationEngine {
    fn clear(&self) {
        self.slab.clear();
    }

    fn look_up_decl_id(&self, index: DeclarationId) -> DeclarationWrapper {
        self.slab.get(*index)
    }

    fn replace_decl_id(&self, index: DeclarationId, wrapper: DeclarationWrapper) {
        self.slab.replace(index, wrapper);
    }

    fn insert(&self, declaration_wrapper: DeclarationWrapper, span: Span) -> DeclarationId {
        DeclarationId::new(self.slab.insert(declaration_wrapper), span)
    }

    fn insert_function(&self, function: ty::TyFunctionDeclaration) -> DeclarationId {
        let span = function.span();
        self.insert(DeclarationWrapper::Function(function), span)
    }

    fn get_function(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyFunctionDeclaration, CompileError> {
        self.slab.get(*index).expect_function(span)
    }

    fn insert_trait(&self, r#trait: ty::TyTraitDeclaration) -> DeclarationId {
        let span = r#trait.name.span();
        self.insert(DeclarationWrapper::Trait(r#trait), span)
    }

    fn get_trait(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyTraitDeclaration, CompileError> {
        self.slab.get(*index).expect_trait(span)
    }

    fn insert_trait_fn(&self, trait_fn: ty::TyTraitFn) -> DeclarationId {
        let span = trait_fn.name.span();
        self.insert(DeclarationWrapper::TraitFn(trait_fn), span)
    }

    fn get_trait_fn(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyTraitFn, CompileError> {
        self.slab.get(*index).expect_trait_fn(span)
    }

    fn insert_impl_trait(&self, impl_trait: ty::TyImplTrait) -> DeclarationId {
        let span = impl_trait.span.clone();
        self.insert(DeclarationWrapper::ImplTrait(impl_trait), span)
    }

    fn get_impl_trait(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyImplTrait, CompileError> {
        self.slab.get(*index).expect_impl_trait(span)
    }

    fn insert_struct(&self, r#struct: ty::TyStructDeclaration) -> DeclarationId {
        let span = r#struct.span();
        self.insert(DeclarationWrapper::Struct(r#struct), span)
    }

    fn get_struct(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyStructDeclaration, CompileError> {
        self.slab.get(*index).expect_struct(span)
    }

    fn insert_storage(&self, storage: ty::TyStorageDeclaration) -> DeclarationId {
        let span = storage.span();
        self.insert(DeclarationWrapper::Storage(storage), span)
    }

    fn get_storage(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyStorageDeclaration, CompileError> {
        self.slab.get(*index).expect_storage(span)
    }

    fn insert_abi(&self, abi: ty::TyAbiDeclaration) -> DeclarationId {
        let span = abi.span.clone();
        self.insert(DeclarationWrapper::Abi(abi), span)
    }

    fn get_abi(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyAbiDeclaration, CompileError> {
        self.slab.get(*index).expect_abi(span)
    }

    fn insert_constant(&self, constant: ty::TyConstantDeclaration) -> DeclarationId {
        let span = constant.name.span();
        DeclarationId::new(
            self.slab
                .insert(DeclarationWrapper::Constant(Box::new(constant))),
            span,
        )
    }

    fn get_constant(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyConstantDeclaration, CompileError> {
        self.slab.get(*index).expect_constant(span)
    }

    fn insert_enum(
        &self,
        enum_decl: ty::TyEnumDeclaration,
        cache_lookup: CacheLookup,
    ) -> DeclarationId {
        let lookup_decl = if cache_lookup == CacheLookup::Enable {
            self.lookup_decl_from_sig(enum_decl.name.clone(), &enum_decl.type_parameters)
        } else {
            None
        };

        if let Some(id) = lookup_decl {
            id
        } else {
            let span = enum_decl.span();
            let id = self.insert(DeclarationWrapper::Enum(enum_decl.clone()), span);
            let decl_sig = DeclarationSignature {
                path: enum_decl.name.span().path().unwrap().clone(),
                name: enum_decl.name,
                type_parameters: enum_decl.type_parameters,
            };
            println!("insert_enum {:?}", decl_sig);
            let mut decls = self.decls.write().unwrap();
            decls.insert(decl_sig, id.clone());
            id
        }
    }

    fn get_enum(
        &self,
        index: DeclarationId,
        span: &Span,
    ) -> Result<ty::TyEnumDeclaration, CompileError> {
        self.slab.get(*index).expect_enum(span)
    }

    fn lookup_decl_from_sig(
        &self,
        name: Ident,
        type_parameters: &[TypeParameter],
    ) -> Option<DeclarationId> {
        let decl_sig = DeclarationSignature {
            path: name.span().path().unwrap().clone(),
            name,
            type_parameters: type_parameters.to_vec(),
        };
        let decls = self.decls.read().unwrap();
        decls.get(&decl_sig).cloned()
    }
}

#[allow(dead_code)]
pub(crate) fn de_print() {
    println!("{}", &*DECLARATION_ENGINE);
}

pub(crate) fn de_clear() {
    DECLARATION_ENGINE.clear();
}

pub fn de_look_up_decl_id(index: DeclarationId) -> DeclarationWrapper {
    DECLARATION_ENGINE.look_up_decl_id(index)
}

pub(crate) fn de_replace_decl_id(index: DeclarationId, wrapper: DeclarationWrapper) {
    DECLARATION_ENGINE.replace_decl_id(index, wrapper)
}

pub(super) fn de_insert(declaration_wrapper: DeclarationWrapper, span: Span) -> DeclarationId {
    DECLARATION_ENGINE.insert(declaration_wrapper, span)
}

pub(crate) fn de_insert_function(function: ty::TyFunctionDeclaration) -> DeclarationId {
    DECLARATION_ENGINE.insert_function(function)
}

pub fn de_get_function(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyFunctionDeclaration, CompileError> {
    DECLARATION_ENGINE.get_function(index, span)
}

pub(crate) fn de_insert_trait(r#trait: ty::TyTraitDeclaration) -> DeclarationId {
    DECLARATION_ENGINE.insert_trait(r#trait)
}

pub fn de_get_trait(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyTraitDeclaration, CompileError> {
    DECLARATION_ENGINE.get_trait(index, span)
}

pub(crate) fn de_insert_trait_fn(trait_fn: ty::TyTraitFn) -> DeclarationId {
    DECLARATION_ENGINE.insert_trait_fn(trait_fn)
}

pub fn de_get_trait_fn(index: DeclarationId, span: &Span) -> Result<ty::TyTraitFn, CompileError> {
    DECLARATION_ENGINE.get_trait_fn(index, span)
}

pub(crate) fn de_insert_impl_trait(impl_trait: ty::TyImplTrait) -> DeclarationId {
    DECLARATION_ENGINE.insert_impl_trait(impl_trait)
}

pub fn de_get_impl_trait(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyImplTrait, CompileError> {
    DECLARATION_ENGINE.get_impl_trait(index, span)
}

pub(crate) fn de_insert_struct(r#struct: ty::TyStructDeclaration) -> DeclarationId {
    DECLARATION_ENGINE.insert_struct(r#struct)
}

pub fn de_get_struct(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyStructDeclaration, CompileError> {
    DECLARATION_ENGINE.get_struct(index, span)
}

pub(crate) fn de_insert_storage(storage: ty::TyStorageDeclaration) -> DeclarationId {
    DECLARATION_ENGINE.insert_storage(storage)
}

pub fn de_get_storage(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyStorageDeclaration, CompileError> {
    DECLARATION_ENGINE.get_storage(index, span)
}

pub(crate) fn de_insert_abi(abi: ty::TyAbiDeclaration) -> DeclarationId {
    DECLARATION_ENGINE.insert_abi(abi)
}

pub fn de_get_abi(index: DeclarationId, span: &Span) -> Result<ty::TyAbiDeclaration, CompileError> {
    DECLARATION_ENGINE.get_abi(index, span)
}

pub(crate) fn de_insert_constant(constant: ty::TyConstantDeclaration) -> DeclarationId {
    DECLARATION_ENGINE.insert_constant(constant)
}

pub fn de_get_constant(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyConstantDeclaration, CompileError> {
    DECLARATION_ENGINE.get_constant(index, span)
}

pub(crate) fn de_insert_enum(
    enum_decl: ty::TyEnumDeclaration,
    cache_lookup: CacheLookup,
) -> DeclarationId {
    DECLARATION_ENGINE.insert_enum(enum_decl, cache_lookup)
}

pub fn de_get_enum(
    index: DeclarationId,
    span: &Span,
) -> Result<ty::TyEnumDeclaration, CompileError> {
    DECLARATION_ENGINE.get_enum(index, span)
}

pub fn de_lookup_enum(name: Ident, type_parameters: &[TypeParameter]) -> Option<DeclarationId> {
    DECLARATION_ENGINE.lookup_decl_from_sig(name, type_parameters)
}
