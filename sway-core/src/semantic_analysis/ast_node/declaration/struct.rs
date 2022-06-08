use crate::{
    error::*, namespace::*, parse_tree::*, semantic_analysis::*, type_engine::*, types::*,
};
use fuels_types::Property;
use std::hash::{Hash, Hasher};
use sway_types::{Ident, Span, Spanned};

#[derive(Clone, Debug, Eq)]
pub struct TypedStructDeclaration {
    pub name: Ident,
    pub fields: Vec<TypedStructField>,
    pub(crate) type_parameters: Vec<TypeParameter>,
    pub(crate) visibility: Visibility,
    pub(crate) span: Span,
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl PartialEq for TypedStructDeclaration {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.fields == other.fields
            && self.type_parameters == other.type_parameters
            && self.visibility == other.visibility
    }
}

impl CopyTypes for TypedStructDeclaration {
    fn copy_types(&mut self, type_mapping: &TypeMapping) {
        self.fields
            .iter_mut()
            .for_each(|x| x.copy_types(type_mapping));
        self.type_parameters
            .iter_mut()
            .for_each(|x| x.copy_types(type_mapping));
    }
}

impl CreateTypeId for TypedStructDeclaration {
    fn create_type_id(&self) -> TypeId {
        insert_type(TypeInfo::Struct {
            name: self.name.clone(),
            fields: self.fields.clone(),
            type_parameters: self.type_parameters.clone(),
        })
    }
}

impl Spanned for TypedStructDeclaration {
    fn span(&self) -> Span {
        self.span.clone()
    }
}

impl MonomorphizeHelper for TypedStructDeclaration {
    type Output = TypedStructDeclaration;

    fn type_parameters(&self) -> &[TypeParameter] {
        &self.type_parameters
    }

    fn name(&self) -> &Ident {
        &self.name
    }

    fn monomorphize_self(self, type_mapping: &TypeMapping, namespace: &mut Items) -> Self::Output {
        monomorphize_decl(self, type_mapping, namespace)
    }
}

impl TypedStructDeclaration {
    pub(crate) fn type_check(
        decl: StructDeclaration,
        namespace: &mut Namespace,
        self_type: TypeId,
    ) -> CompileResult<TypedStructDeclaration> {
        let mut warnings = vec![];
        let mut errors = vec![];

        let StructDeclaration {
            name,
            fields,
            mut type_parameters,
            visibility,
            span,
        } = decl;

        // create a namespace for the decl, used to create a scope for generics
        let mut namespace = namespace.clone();

        // type check the type parameters and insert them into the namespace
        for type_parameter in type_parameters.iter_mut() {
            check!(
                TypeParameter::type_check(type_parameter, &mut namespace),
                return err(warnings, errors),
                warnings,
                errors
            );
        }

        // type check the fields
        let mut new_fields = vec![];
        for field in fields.into_iter() {
            new_fields.push(check!(
                TypedStructField::type_check(field, &mut namespace, self_type),
                return err(warnings, errors),
                warnings,
                errors
            ));
        }

        // create the struct decl
        let decl = TypedStructDeclaration {
            name,
            type_parameters,
            fields: new_fields,
            visibility,
            span,
        };

        ok(decl, warnings, errors)
    }

    pub(crate) fn expect_field(&self, field_to_access: &Ident) -> CompileResult<&TypedStructField> {
        let warnings = vec![];
        let mut errors = vec![];
        match self
            .fields
            .iter()
            .find(|TypedStructField { name, .. }| name.as_str() == field_to_access.as_str())
        {
            Some(field) => ok(field, warnings, errors),
            None => {
                errors.push(CompileError::FieldNotFound {
                    available_fields: self
                        .fields
                        .iter()
                        .map(|TypedStructField { name, .. }| name.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    field_name: field_to_access.clone(),
                    struct_name: self.name.clone(),
                });
                err(warnings, errors)
            }
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct TypedStructField {
    pub name: Ident,
    pub r#type: TypeId,
    pub(crate) span: Span,
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl Hash for TypedStructField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        look_up_type_id(self.r#type).hash(state);
    }
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl PartialEq for TypedStructField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && look_up_type_id(self.r#type) == look_up_type_id(other.r#type)
    }
}

impl CopyTypes for TypedStructField {
    fn copy_types(&mut self, type_mapping: &TypeMapping) {
        self.r#type.update_type(type_mapping, &self.span);
    }
}

impl ToJsonAbi for TypedStructField {
    type Output = Property;

    fn generate_json_abi(&self) -> Self::Output {
        Property {
            name: self.name.to_string(),
            type_field: self.r#type.json_abi_str(),
            components: self.r#type.generate_json_abi(),
        }
    }
}

impl TypedStructField {
    pub(crate) fn type_check(
        field: StructField,
        namespace: &mut Namespace,
        self_type: TypeId,
    ) -> CompileResult<TypedStructField> {
        let mut warnings = vec![];
        let mut errors = vec![];
        let r#type = check!(
            namespace.resolve_type_with_self(
                field.r#type,
                self_type,
                &field.type_span,
                EnforceTypeArguments::Yes
            ),
            insert_type(TypeInfo::ErrorRecovery),
            warnings,
            errors,
        );
        let field = TypedStructField {
            name: field.name,
            r#type,
            span: field.span,
        };
        ok(field, warnings, errors)
    }
}
