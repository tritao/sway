use super::*;
use crate::TypedDeclaration;
use crate::concurrent_slab::ConcurrentSlabFunction;
use lazy_static::lazy_static;

lazy_static! {
    static ref FUNCTION_ENGINE: FunctionEngine = FunctionEngine::default();
}

#[derive(Debug, Default)]
pub(crate) struct FunctionEngine {
    slab: ConcurrentSlabFunction<TypedDeclaration>,
}

impl FunctionEngine {
    pub fn insert_function(&self, decl: TypedDeclaration) -> FunctionId {
        self.slab.insert(decl)
    }

    pub fn look_up_function_id_raw(&self, id: FunctionId) -> TypedDeclaration {
        self.slab.get(id)
    }

    pub fn look_up_function_id(&self, id: FunctionId) -> TypedDeclaration {
        match self.slab.get(id) {
            TypedDeclaration::FunctionRef(other) => self.look_up_function_id(other),
            ty => ty,
        }
    }
}

pub fn insert_function(decl: TypedDeclaration) -> FunctionId {
    FUNCTION_ENGINE.insert_function(decl)
}

pub(crate) fn look_up_function_id(id: FunctionId) -> TypedDeclaration {
    FUNCTION_ENGINE.look_up_function_id(id)
}

pub(crate) fn look_up_function_id_raw(id: FunctionId) -> TypedDeclaration {
    FUNCTION_ENGINE.look_up_function_id_raw(id)
}


