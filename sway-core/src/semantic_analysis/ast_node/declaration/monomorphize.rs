use sway_types::{Ident, Span, Spanned};

use crate::{
    error::*, namespace::*, type_engine::*, CompileError, CompileResult, TypeArgument, TypeInfo,
    TypeParameter,
};

use super::CreateTypeId;

/// This type is used to denote if, during monomorphization, the compiler
/// should enforce that type arguments be provided. An example of that
/// might be this:
///
/// ```ignore
/// struct Point<T> {
///   x: u64,
///   y: u64
/// }
///
/// fn add<T>(p1: Point<T>, p2: Point<T>) -> Point<T> {
///   Point {
///     x: p1.x + p2.x,
///     y: p1.y + p2.y
///   }
/// }
/// ```
///
/// `EnforeTypeArguments` would require that the type annotations
/// for `p1` and `p2` contain `<...>`. This is to avoid ambiguous definitions:
///
/// ```ignore
/// fn add(p1: Point, p2: Point) -> Point {
///   Point {
///     x: p1.x + p2.x,
///     y: p1.y + p2.y
///   }
/// }
/// ```
#[derive(Clone, Copy)]
pub(crate) enum EnforceTypeArguments {
    Yes,
    No,
}

pub(crate) trait Monomorphize {
    type Output;

    fn monomorphize_with_self(
        self,
        type_arguments: Vec<TypeArgument>,
        enforce_type_arguments: EnforceTypeArguments,
        self_type: TypeId,
        call_site_span: Option<&Span>,
        namespace: &mut Root,
        module_path: &Path,
    ) -> CompileResult<Self::Output>;

    fn monomorphize_without_self(
        self,
        type_arguments: Vec<TypeArgument>,
        enforce_type_arguments: EnforceTypeArguments,
        call_site_span: Option<&Span>,
        namespace: &mut Root,
        module_path: &Path,
    ) -> CompileResult<Self::Output>;
}

impl<T> Monomorphize for T
where
    T: MonomorphizeHelper<Output = T> + Spanned,
{
    type Output = T;

    fn monomorphize_with_self(
        self,
        type_arguments: Vec<TypeArgument>,
        enforce_type_arguments: EnforceTypeArguments,
        self_type: TypeId,
        call_site_span: Option<&Span>,
        namespace: &mut Root,
        module_path: &Path,
    ) -> CompileResult<Self::Output> {
        let inner_function = |decl: &T,
                              type_arguments: Vec<TypeArgument>,
                              namespace: &mut Root,
                              module_path: &Path| {
            resolve_types_with_self(
                decl,
                type_arguments,
                enforce_type_arguments,
                self_type,
                namespace,
                module_path,
            )
        };
        monomorphize_inner(
            self,
            type_arguments,
            enforce_type_arguments,
            call_site_span,
            namespace,
            module_path,
            inner_function,
        )
    }

    fn monomorphize_without_self(
        self,
        type_arguments: Vec<TypeArgument>,
        enforce_type_arguments: EnforceTypeArguments,
        call_site_span: Option<&Span>,
        namespace: &mut Root,
        module_path: &Path,
    ) -> CompileResult<Self::Output> {
        monomorphize_inner(
            self,
            type_arguments,
            enforce_type_arguments,
            call_site_span,
            namespace,
            module_path,
            resolve_types_without_self,
        )
    }
}

fn monomorphize_inner<T, F>(
    decl: T,
    type_arguments: Vec<TypeArgument>,
    enforce_type_arguments: EnforceTypeArguments,
    call_site_span: Option<&Span>,
    namespace: &mut Root,
    module_path: &Path,
    inner_function: F,
) -> CompileResult<T>
where
    T: MonomorphizeHelper<Output = T> + Spanned,
    F: FnOnce(&T, Vec<TypeArgument>, &mut Root, &Path) -> CompileResult<TypeMapping>,
{
    let mut warnings = vec![];
    let mut errors = vec![];
    match (decl.type_parameters().is_empty(), type_arguments.is_empty()) {
        (true, true) => ok(decl, vec![], vec![]),
        (false, true) => {
            if let EnforceTypeArguments::Yes = enforce_type_arguments {
                let name_span = decl.name().span();
                errors.push(CompileError::NeedsTypeArguments {
                    name: decl.name().clone(),
                    span: call_site_span.unwrap_or(&name_span).clone(),
                });
                return err(warnings, errors);
            }
            let type_mapping = insert_type_parameters(decl.type_parameters());
            let module = check!(
                namespace.check_submodule_mut(module_path),
                return err(warnings, errors),
                warnings,
                errors
            );
            let new_decl = decl.monomorphize_self(&type_mapping, module);
            ok(new_decl, warnings, errors)
        }
        (true, false) => {
            let type_arguments_span = type_arguments
                .iter()
                .map(|x| x.span.clone())
                .reduce(Span::join)
                .unwrap_or_else(|| decl.span());
            errors.push(CompileError::DoesNotTakeTypeArguments {
                name: decl.name().clone(),
                span: type_arguments_span,
            });
            err(warnings, errors)
        }
        (false, false) => {
            let type_arguments_span = type_arguments
                .iter()
                .map(|x| x.span.clone())
                .reduce(Span::join)
                .unwrap_or_else(|| decl.span());
            if decl.type_parameters().len() != type_arguments.len() {
                errors.push(CompileError::IncorrectNumberOfTypeArguments {
                    given: type_arguments.len(),
                    expected: decl.type_parameters().len(),
                    span: type_arguments_span,
                });
                return err(warnings, errors);
            }
            let type_mapping = check!(
                inner_function(&decl, type_arguments, namespace, module_path),
                vec!(),
                warnings,
                errors
            );
            let module = check!(
                namespace.check_submodule_mut(module_path),
                return err(warnings, errors),
                warnings,
                errors
            );
            let new_decl = decl.monomorphize_self(&type_mapping, module);
            ok(new_decl, warnings, errors)
        }
    }
}

fn resolve_types_with_self<T>(
    decl: &T,
    mut type_arguments: Vec<TypeArgument>,
    enforce_type_arguments: EnforceTypeArguments,
    self_type: TypeId,
    namespace: &mut Root,
    module_path: &Path,
) -> CompileResult<TypeMapping>
where
    T: MonomorphizeHelper<Output = T>,
{
    let mut warnings = vec![];
    let mut errors = vec![];
    for type_argument in type_arguments.iter_mut() {
        type_argument.type_id = check!(
            namespace.resolve_type_with_self(
                look_up_type_id(type_argument.type_id),
                self_type,
                &type_argument.span,
                enforce_type_arguments,
                module_path,
            ),
            insert_type(TypeInfo::ErrorRecovery),
            warnings,
            errors
        );
    }
    if decl.name().as_str() == "DoubleIdentity" {
        println!(
            "-------------\n\ntype arguments: <{}>",
            type_arguments
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let type_mapping = insert_type_parameters(decl.type_parameters());
    for ((_, interim_type), type_argument) in type_mapping.iter().zip(type_arguments.iter()) {
        let (mut new_warnings, new_errors) = unify_with_self(
            *interim_type,
            type_argument.type_id,
            self_type,
            &type_argument.span,
            "Type argument is not assignable to generic type parameter.",
        );
        warnings.append(&mut new_warnings);
        errors.append(&mut new_errors.into_iter().map(|x| x.into()).collect());
    }
    ok(type_mapping, warnings, errors)
}

fn resolve_types_without_self<T>(
    decl: &T,
    mut type_arguments: Vec<TypeArgument>,
    namespace: &mut Root,
    module_path: &Path,
) -> CompileResult<TypeMapping>
where
    T: MonomorphizeHelper<Output = T>,
{
    let mut warnings = vec![];
    let mut errors = vec![];
    for type_argument in type_arguments.iter_mut() {
        type_argument.type_id = check!(
            namespace
                .resolve_type_without_self(look_up_type_id(type_argument.type_id), module_path,),
            insert_type(TypeInfo::ErrorRecovery),
            warnings,
            errors
        );
    }
    if decl.name().as_str() == "DoubleIdentity" {
        println!(
            "-------------\n\ntype arguments: <{}>",
            type_arguments
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let type_mapping = insert_type_parameters(decl.type_parameters());
    for ((_, interim_type), type_argument) in type_mapping.iter().zip(type_arguments.iter()) {
        let (mut new_warnings, new_errors) = unify(
            *interim_type,
            type_argument.type_id,
            &type_argument.span,
            "Type argument is not assignable to generic type parameter.",
        );
        warnings.append(&mut new_warnings);
        errors.append(&mut new_errors.into_iter().map(|x| x.into()).collect());
    }
    ok(type_mapping, warnings, errors)
}

pub(crate) trait MonomorphizeHelper {
    type Output;

    fn type_parameters(&self) -> &[TypeParameter];
    fn name(&self) -> &Ident;
    fn monomorphize_self(self, type_mapping: &TypeMapping, namespace: &mut Items) -> Self::Output;
}

pub(crate) fn monomorphize_decl<T>(decl: T, type_mapping: &TypeMapping, namespace: &mut Items) -> T
where
    T: CopyTypes + CreateTypeId,
{
    let old_type_id = decl.create_type_id();
    let mut new_decl = decl;
    new_decl.copy_types(type_mapping);
    namespace.copy_methods_to_type(
        look_up_type_id(old_type_id),
        look_up_type_id(new_decl.create_type_id()),
        type_mapping,
    );
    new_decl
}
