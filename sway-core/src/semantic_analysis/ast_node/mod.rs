mod code_block;
pub mod declaration;
pub mod expression;
pub mod mode;
mod return_statement;
pub mod while_loop;

use std::fmt;

pub(crate) use code_block::*;
pub use declaration::*;
pub(crate) use expression::*;
pub(crate) use mode::*;
pub(crate) use return_statement::*;
pub(crate) use while_loop::*;

use crate::{
    error::*, parse_tree::*, semantic_analysis::*, style::*, type_engine::*,
    types::DeterministicallyAborts, AstNode, AstNodeContent, Ident, ReturnStatement,
};

use sway_types::{span::Span, state::StateIndex, Spanned};

use derivative::Derivative;

/// whether or not something is constantly evaluatable (if the result is known at compile
/// time)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum IsConstant {
    Yes,
    No,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedAstNodeContent {
    ReturnStatement(TypedReturnStatement),
    Declaration(TypedDeclaration),
    Expression(TypedExpression),
    ImplicitReturnExpression(TypedExpression),
    WhileLoop(TypedWhileLoop),
    // a no-op node used for something that just issues a side effect, like an import statement.
    SideEffect,
}

impl UnresolvedTypeCheck for TypedAstNodeContent {
    fn check_for_unresolved_types(&self) -> Vec<CompileError> {
        use TypedAstNodeContent::*;
        match self {
            ReturnStatement(stmt) => stmt.expr.check_for_unresolved_types(),
            Declaration(decl) => decl.check_for_unresolved_types(),
            Expression(expr) => expr.check_for_unresolved_types(),
            ImplicitReturnExpression(expr) => expr.check_for_unresolved_types(),
            WhileLoop(lo) => {
                let mut condition = lo.condition.check_for_unresolved_types();
                let mut body = lo
                    .body
                    .contents
                    .iter()
                    .flat_map(TypedAstNode::check_for_unresolved_types)
                    .collect();
                condition.append(&mut body);
                condition
            }
            SideEffect => vec![],
        }
    }
}

#[derive(Clone, Debug, Eq, Derivative)]
#[derivative(PartialEq)]
pub struct TypedAstNode {
    pub content: TypedAstNodeContent,
    #[derivative(PartialEq = "ignore")]
    pub(crate) span: Span,
}

impl fmt::Display for TypedAstNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TypedAstNodeContent::*;
        let text = match &self.content {
            ReturnStatement(TypedReturnStatement { ref expr }) => {
                format!("return {}", expr)
            }
            Declaration(ref typed_decl) => typed_decl.to_string(),
            Expression(exp) => exp.to_string(),
            ImplicitReturnExpression(exp) => format!("return {}", exp),
            WhileLoop(w_loop) => w_loop.to_string(),
            SideEffect => "".into(),
        };
        f.write_str(&text)
    }
}

impl CopyTypes for TypedAstNode {
    fn copy_types(&mut self, type_mapping: &TypeMapping) {
        match self.content {
            TypedAstNodeContent::ReturnStatement(ref mut ret_stmt) => {
                ret_stmt.copy_types(type_mapping)
            }
            TypedAstNodeContent::ImplicitReturnExpression(ref mut exp) => {
                exp.copy_types(type_mapping)
            }
            TypedAstNodeContent::Declaration(ref mut decl) => decl.copy_types(type_mapping),
            TypedAstNodeContent::Expression(ref mut expr) => expr.copy_types(type_mapping),
            TypedAstNodeContent::WhileLoop(TypedWhileLoop {
                ref mut condition,
                ref mut body,
            }) => {
                condition.copy_types(type_mapping);
                body.copy_types(type_mapping);
            }
            TypedAstNodeContent::SideEffect => (),
        }
    }
}

impl UnresolvedTypeCheck for TypedAstNode {
    fn check_for_unresolved_types(&self) -> Vec<CompileError> {
        self.content.check_for_unresolved_types()
    }
}

impl DeterministicallyAborts for TypedAstNode {
    fn deterministically_aborts(&self) -> bool {
        use TypedAstNodeContent::*;
        match &self.content {
            ReturnStatement(_) => true,
            Declaration(_) => false,
            Expression(exp) | ImplicitReturnExpression(exp) => exp.deterministically_aborts(),
            WhileLoop(TypedWhileLoop { condition, body }) => {
                condition.deterministically_aborts() || body.deterministically_aborts()
            }
            SideEffect => false,
        }
    }
}

impl TypedAstNode {
    /// Returns `true` if this AST node will be exported in a library, i.e. it is a public declaration.
    pub(crate) fn is_public(&self) -> bool {
        use TypedAstNodeContent::*;
        match &self.content {
            Declaration(decl) => decl.visibility().is_public(),
            ReturnStatement(_)
            | Expression(_)
            | WhileLoop(_)
            | SideEffect
            | ImplicitReturnExpression(_) => false,
        }
    }

    /// Naive check to see if this node is a function declaration of a function called `main` if
    /// the [TreeType] is Script or Predicate.
    pub(crate) fn is_main_function(&self, tree_type: TreeType) -> bool {
        match &self {
            TypedAstNode {
                content:
                    TypedAstNodeContent::Declaration(TypedDeclaration::FunctionDeclaration(
                        TypedFunctionDeclaration { name, .. },
                    )),
                ..
            } if name.as_str() == crate::constants::DEFAULT_ENTRY_POINT_FN_NAME => {
                matches!(tree_type, TreeType::Script | TreeType::Predicate)
            }
            _ => false,
        }
    }

    /// recurse into `self` and get any return statements -- used to validate that all returns
    /// do indeed return the correct type
    /// This does _not_ extract implicit return statements as those are not control flow! This is
    /// _only_ for explicit returns.
    pub(crate) fn gather_return_statements(&self) -> Vec<&TypedReturnStatement> {
        match &self.content {
            TypedAstNodeContent::ReturnStatement(ref stmt) => vec![stmt],
            TypedAstNodeContent::ImplicitReturnExpression(ref exp) => {
                exp.gather_return_statements()
            }
            TypedAstNodeContent::WhileLoop(TypedWhileLoop {
                ref condition,
                ref body,
                ..
            }) => {
                let mut buf = condition.gather_return_statements();
                for node in &body.contents {
                    buf.append(&mut node.gather_return_statements())
                }
                buf
            }
            // assignments and  reassignments can happen during control flow and can abort
            TypedAstNodeContent::Declaration(TypedDeclaration::VariableDeclaration(
                TypedVariableDeclaration { body, .. },
            )) => body.gather_return_statements(),
            TypedAstNodeContent::Declaration(TypedDeclaration::Reassignment(
                TypedReassignment { rhs, .. },
            )) => rhs.gather_return_statements(),
            TypedAstNodeContent::Expression(exp) => exp.gather_return_statements(),
            TypedAstNodeContent::SideEffect | TypedAstNodeContent::Declaration(_) => vec![],
        }
    }

    fn type_info(&self) -> TypeInfo {
        // return statement should be ()
        use TypedAstNodeContent::*;
        match &self.content {
            ReturnStatement(_) | Declaration(_) => TypeInfo::Tuple(Vec::new()),
            Expression(TypedExpression { return_type, .. }) => {
                crate::type_engine::look_up_type_id(*return_type)
            }
            ImplicitReturnExpression(TypedExpression { return_type, .. }) => {
                crate::type_engine::look_up_type_id(*return_type)
            }
            WhileLoop(_) | SideEffect => TypeInfo::Tuple(Vec::new()),
        }
    }

    pub(crate) fn type_check(
        arguments: TypeCheckArguments<'_, AstNode>,
    ) -> CompileResult<TypedAstNode> {
        let TypeCheckArguments {
            checkee: node,
            namespace,
            return_type_annotation,
            help_text,
            self_type,
            opts,
            ..
        } = arguments;
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // A little utility used to check an ascribed type matches its associated expression.
        let mut type_check_ascribed_expr =
            |namespace: &mut Namespace, type_ascription: TypeInfo, value| {
                let type_id = check!(
                    namespace.resolve_type_with_self(
                        type_ascription,
                        self_type,
                        &node.span,
                        EnforceTypeArguments::No
                    ),
                    insert_type(TypeInfo::ErrorRecovery),
                    warnings,
                    errors,
                );
                TypedExpression::type_check(TypeCheckArguments {
                    checkee: value,
                    namespace,
                    return_type_annotation: type_id,
                    help_text: "This declaration's type annotation  does \
                     not match up with the assigned expression's type.",
                    self_type,
                    mode: Mode::NonAbi,
                    opts,
                })
            };

        let content = match node.content.clone() {
            AstNodeContent::UseStatement(a) => {
                let path = if a.is_absolute {
                    a.call_path.clone()
                } else {
                    namespace.find_module_path(&a.call_path)
                };
                let mut res = match a.import_type {
                    ImportType::Star => namespace.star_import(&path),
                    ImportType::SelfImport => namespace.self_import(&path, a.alias),
                    ImportType::Item(s) => namespace.item_import(&path, &s, a.alias),
                };
                warnings.append(&mut res.warnings);
                errors.append(&mut res.errors);
                TypedAstNodeContent::SideEffect
            }
            AstNodeContent::IncludeStatement(_) => TypedAstNodeContent::SideEffect,
            AstNodeContent::Declaration(a) => {
                TypedAstNodeContent::Declaration(match a {
                    Declaration::VariableDeclaration(VariableDeclaration {
                        name,
                        type_ascription,
                        type_ascription_span,
                        body,
                        is_mutable,
                    }) => {
                        check_if_name_is_invalid(&name).ok(&mut warnings, &mut errors);
                        let type_ascription_span = match type_ascription_span {
                            Some(type_ascription_span) => type_ascription_span,
                            None => name.span(),
                        };
                        let type_ascription = check!(
                            namespace.resolve_type_with_self(
                                type_ascription,
                                self_type,
                                &type_ascription_span,
                                EnforceTypeArguments::Yes,
                            ),
                            insert_type(TypeInfo::ErrorRecovery),
                            warnings,
                            errors
                        );
                        let result = {
                            TypedExpression::type_check(TypeCheckArguments {
                                checkee: body,
                                namespace,
                                return_type_annotation: type_ascription,
                                help_text: "Variable declaration's type annotation does \
                 not match up with the assigned expression's type.",
                                self_type,
                                mode: Mode::NonAbi,
                                opts,
                            })
                        };
                        let body =
                            check!(result, error_recovery_expr(name.span()), warnings, errors);
                        let typed_var_decl =
                            TypedDeclaration::VariableDeclaration(TypedVariableDeclaration {
                                name: name.clone(),
                                body,
                                is_mutable: is_mutable.into(),
                                const_decl_origin: false,
                                type_ascription,
                            });
                        namespace.insert_symbol(name, typed_var_decl.clone());
                        typed_var_decl
                    }
                    Declaration::ConstantDeclaration(ConstantDeclaration {
                        name,
                        type_ascription,
                        value,
                        visibility,
                    }) => {
                        let result =
                            type_check_ascribed_expr(namespace, type_ascription.clone(), value);
                        is_screaming_snake_case(&name).ok(&mut warnings, &mut errors);
                        let value =
                            check!(result, error_recovery_expr(name.span()), warnings, errors);
                        let typed_const_decl =
                            TypedDeclaration::VariableDeclaration(TypedVariableDeclaration {
                                name: name.clone(),
                                body: value,
                                is_mutable: if visibility.is_public() {
                                    VariableMutability::ExportedConst
                                } else {
                                    VariableMutability::Immutable
                                },
                                const_decl_origin: true,
                                type_ascription: insert_type(type_ascription),
                            });
                        namespace.insert_symbol(name, typed_const_decl.clone());
                        typed_const_decl
                    }
                    Declaration::EnumDeclaration(decl) => {
                        let decl = check!(
                            TypedEnumDeclaration::type_check(decl, namespace, self_type),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        let name = decl.name.clone();
                        let decl = TypedDeclaration::EnumDeclaration(decl);
                        let _ = check!(
                            namespace.insert_symbol(name, decl.clone()),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        decl
                    }
                    Declaration::FunctionDeclaration(fn_decl) => {
                        for type_parameter in fn_decl.type_parameters.iter() {
                            if !type_parameter.trait_constraints.is_empty() {
                                errors.push(CompileError::WhereClauseNotYetSupported {
                                    span: type_parameter.name_ident.span(),
                                });
                                break;
                            }
                        }

                        let decl = check!(
                            TypedFunctionDeclaration::type_check(TypeCheckArguments {
                                checkee: fn_decl.clone(),
                                namespace,
                                return_type_annotation: insert_type(TypeInfo::Unknown),
                                help_text,
                                self_type,
                                mode: Mode::NonAbi,
                                opts
                            }),
                            error_recovery_function_declaration(fn_decl),
                            warnings,
                            errors
                        );
                        namespace.insert_symbol(
                            decl.name.clone(),
                            TypedDeclaration::FunctionDeclaration(decl.clone()),
                        );
                        TypedDeclaration::FunctionDeclaration(decl)
                    }
                    Declaration::TraitDeclaration(trait_decl) => {
                        is_upper_camel_case(&trait_decl.name).ok(&mut warnings, &mut errors);
                        let decl = check!(
                            TypedTraitDeclaration::type_check(TypeCheckArguments {
                                checkee: trait_decl,
                                namespace,
                                return_type_annotation: insert_type(TypeInfo::SelfType),
                                help_text: Default::default(),
                                self_type,
                                mode: Mode::NonAbi,
                                opts
                            }),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        namespace.insert_symbol(
                            decl.name.clone(),
                            TypedDeclaration::TraitDeclaration(decl.clone()),
                        );
                        TypedDeclaration::TraitDeclaration(decl)
                    }
                    Declaration::Reassignment(Reassignment { lhs, rhs, span }) => {
                        check!(
                            reassignment(
                                TypeCheckArguments {
                                    checkee: (lhs, rhs),
                                    namespace,
                                    self_type,
                                    // this is unused by `reassignment`
                                    return_type_annotation: insert_type(TypeInfo::Unknown),
                                    help_text: Default::default(),
                                    mode: Mode::NonAbi,
                                    opts,
                                },
                                span,
                            ),
                            return err(warnings, errors),
                            warnings,
                            errors
                        )
                    }
                    Declaration::ImplTrait(impl_trait) => {
                        let impl_trait = check!(
                            TypedImplTrait::type_check(impl_trait, namespace, opts),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        TypedDeclaration::ImplTrait(impl_trait)
                    }
                    Declaration::ImplSelf(ImplSelf {
                        functions,
                        type_implementing_for,
                        block_span,
                        mut type_parameters,
                        ..
                    }) => {
                        for type_parameter in type_parameters.iter() {
                            if !type_parameter.trait_constraints.is_empty() {
                                errors.push(CompileError::WhereClauseNotYetSupported {
                                    span: type_parameter.name_ident.span(),
                                });
                                break;
                            }
                        }

                        // create the namespace for the impl
                        let mut impl_namespace = namespace.clone();

                        // insert type parameters as Unknown types
                        let type_mapping = insert_type_parameters(&type_parameters);

                        // update the types in the type parameters
                        for type_parameter in type_parameters.iter_mut() {
                            check!(
                                type_parameter.update_types(
                                    &type_mapping,
                                    &mut impl_namespace,
                                    self_type
                                ),
                                return err(warnings, errors),
                                warnings,
                                errors
                            );
                        }

                        // check to see if the type parameters shadow one another
                        for type_parameter in type_parameters.iter() {
                            let type_parameter_decl =
                                TypedDeclaration::GenericTypeForFunctionScope {
                                    name: type_parameter.name_ident.clone(),
                                    type_id: type_parameter.type_id,
                                };
                            check!(
                                impl_namespace.insert_symbol(
                                    type_parameter.name_ident.clone(),
                                    type_parameter_decl
                                ),
                                continue,
                                warnings,
                                errors
                            );
                        }

                        // for type_parameter in type_parameters.iter() {
                        //     impl_namespace.insert_symbol(
                        //         type_parameter.name_ident.clone(),
                        //         type_parameter.into(),
                        //     );
                        // }

                        // Resolve the Self type as it's most likely still 'Custom' and use the
                        // resolved type for self instead.
                        let implementing_for_type_id = check!(
                            impl_namespace.resolve_type_without_self(type_implementing_for),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        let type_implementing_for = look_up_type_id(implementing_for_type_id);
                        let mut functions_buf: Vec<TypedFunctionDeclaration> = vec![];
                        for fn_decl in functions.into_iter() {
                            functions_buf.push(check!(
                                TypedFunctionDeclaration::type_check(TypeCheckArguments {
                                    checkee: fn_decl,
                                    namespace: &mut impl_namespace,
                                    return_type_annotation: insert_type(TypeInfo::Unknown),
                                    help_text: "",
                                    self_type: implementing_for_type_id,
                                    mode: Mode::NonAbi,
                                    opts,
                                }),
                                continue,
                                warnings,
                                errors
                            ));
                        }
                        let trait_name = CallPath {
                            prefixes: vec![],
                            suffix: Ident::new_with_override("r#Self", block_span.clone()),
                            is_absolute: false,
                        };
                        namespace.insert_trait_implementation(
                            trait_name.clone(),
                            type_implementing_for.clone(),
                            functions_buf.clone(),
                        );
                        let impl_trait = TypedImplTrait {
                            trait_name,
                            span: block_span,
                            methods: functions_buf,
                            type_implementing_for,
                        };
                        TypedDeclaration::ImplTrait(impl_trait)
                    }
                    Declaration::StructDeclaration(decl) => {
                        let decl = check!(
                            TypedStructDeclaration::type_check(decl, namespace, self_type),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        let name = decl.name.clone();
                        let decl = TypedDeclaration::StructDeclaration(decl);
                        // insert the struct decl into namespace
                        let _ = check!(
                            namespace.insert_symbol(name, decl.clone()),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        decl
                    }
                    Declaration::AbiDeclaration(AbiDeclaration {
                        name,
                        interface_surface,
                        methods,
                        span,
                    }) => {
                        // type check the interface surface and methods
                        // We don't want the user to waste resources by contract calling
                        // themselves, and we don't want to do more work in the compiler,
                        // so we don't support the case of calling a contract's own interface
                        // from itself. This is by design.
                        let mut new_interface_surface = vec![];
                        let self_type = insert_type(TypeInfo::SelfType);
                        for trait_fn in interface_surface.into_iter() {
                            new_interface_surface.push(check!(
                                TypedTraitFn::type_check(
                                    trait_fn,
                                    namespace,
                                    insert_type(TypeInfo::SelfType)
                                ),
                                continue,
                                warnings,
                                errors
                            ));
                        }

                        // type check these for errors but don't actually use them yet -- the real
                        // ones will be type checked with proper symbols when the ABI is implemented
                        for method in methods.iter() {
                            check!(
                                TypedFunctionDeclaration::type_check(TypeCheckArguments {
                                    checkee: method.clone(),
                                    namespace,
                                    return_type_annotation: insert_type(TypeInfo::Unknown),
                                    help_text: Default::default(),
                                    self_type,
                                    mode: Mode::NonAbi,
                                    opts
                                }),
                                continue,
                                warnings,
                                errors
                            );
                        }
                        // let _methods = check!(
                        //     type_check_trait_methods(methods.clone(), namespace, self_type,),
                        //     vec![],
                        //     warnings,
                        //     errors
                        // );

                        let decl = TypedDeclaration::AbiDeclaration(TypedAbiDeclaration {
                            interface_surface: new_interface_surface,
                            methods,
                            name: name.clone(),
                            span,
                        });
                        namespace.insert_symbol(name, decl.clone());
                        decl
                    }
                    Declaration::StorageDeclaration(StorageDeclaration { span, fields }) => {
                        let mut fields_buf = Vec::with_capacity(fields.len());
                        for StorageField { name, r#type } in fields {
                            let r#type = check!(
                                namespace.resolve_type_without_self(r#type),
                                return err(warnings, errors),
                                warnings,
                                errors
                            );
                            fields_buf.push(TypedStorageField::new(name, r#type, span.clone()));
                        }

                        let decl = TypedStorageDeclaration::new(fields_buf, span);
                        // insert the storage declaration into the symbols
                        // if there already was one, return an error that duplicate storage

                        // declarations are not allowed
                        check!(
                            namespace.set_storage_declaration(decl.clone()),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        TypedDeclaration::StorageDeclaration(decl)
                    }
                })
            }
            AstNodeContent::Expression(a) => {
                let inner = check!(
                    TypedExpression::type_check(TypeCheckArguments {
                        checkee: a.clone(),
                        namespace,
                        return_type_annotation: insert_type(TypeInfo::Unknown),
                        help_text: Default::default(),
                        self_type,
                        mode: Mode::NonAbi,
                        opts
                    }),
                    error_recovery_expr(a.span()),
                    warnings,
                    errors
                );
                TypedAstNodeContent::Expression(inner)
            }
            AstNodeContent::ReturnStatement(ReturnStatement { expr }) => {
                TypedAstNodeContent::ReturnStatement(TypedReturnStatement {
                    expr: check!(
                        TypedExpression::type_check(TypeCheckArguments {
                            checkee: expr.clone(),
                            namespace,
                            // we use "unknown" here because return statements do not
                            // necessarily follow the type annotation of their immediate
                            // surrounding context. Because a return statement is control flow
                            // that breaks out to the nearest function, we need to type check
                            // it against the surrounding function.
                            // That is impossible here, as we don't have that information. It
                            // is the responsibility of the function declaration to type check
                            // all return statements contained within it.
                            return_type_annotation: insert_type(TypeInfo::Unknown),
                            help_text:
                                "Returned value must match up with the function return type \
                             annotation.",
                            self_type,
                            mode: Mode::NonAbi,
                            opts
                        }),
                        error_recovery_expr(expr.span()),
                        warnings,
                        errors
                    ),
                })
            }
            AstNodeContent::ImplicitReturnExpression(expr) => {
                let typed_expr = check!(
                    TypedExpression::type_check(TypeCheckArguments {
                        checkee: expr.clone(),
                        namespace,
                        return_type_annotation,
                        help_text: "Implicit return must match up with block's type.",
                        self_type,
                        mode: Mode::NonAbi,
                        opts,
                    }),
                    error_recovery_expr(expr.span()),
                    warnings,
                    errors
                );
                TypedAstNodeContent::ImplicitReturnExpression(typed_expr)
            }
            AstNodeContent::WhileLoop(WhileLoop { condition, body }) => {
                let typed_condition = check!(
                    TypedExpression::type_check(TypeCheckArguments {
                        checkee: condition,
                        namespace,
                        return_type_annotation: insert_type(TypeInfo::Boolean),
                        help_text: "A while loop's loop condition must be a boolean expression.",
                        self_type,
                        mode: Mode::NonAbi,
                        opts
                    }),
                    return err(warnings, errors),
                    warnings,
                    errors
                );
                let (typed_body, _block_implicit_return) = check!(
                    TypedCodeBlock::type_check(TypeCheckArguments {
                        checkee: body,
                        namespace,
                        return_type_annotation: insert_type(TypeInfo::Tuple(Vec::new())),
                        help_text: "A while loop's loop body cannot implicitly return a value.Try \
                         assigning it to a mutable variable declared outside of the loop \
                         instead.",
                        self_type,
                        mode: Mode::NonAbi,
                        opts,
                    }),
                    (
                        TypedCodeBlock { contents: vec![] },
                        insert_type(TypeInfo::Tuple(Vec::new()))
                    ),
                    warnings,
                    errors
                );
                TypedAstNodeContent::WhileLoop(TypedWhileLoop {
                    condition: typed_condition,
                    body: typed_body,
                })
            }
        };

        let node = TypedAstNode {
            content,
            span: node.span.clone(),
        };

        if let TypedAstNode {
            content: TypedAstNodeContent::Expression(TypedExpression { .. }),
            ..
        } = node
        {
            let warning = Warning::UnusedReturnValue {
                r#type: Box::new(node.type_info()),
            };
            assert_or_warn!(
                node.type_info().is_unit() || node.type_info() == TypeInfo::ErrorRecovery,
                warnings,
                node.span.clone(),
                warning
            );
        }

        ok(node, warnings, errors)
    }
}

fn reassignment(
    arguments: TypeCheckArguments<'_, (ReassignmentTarget, Expression)>,
    span: Span,
) -> CompileResult<TypedDeclaration> {
    let TypeCheckArguments {
        checkee: (lhs, rhs),
        namespace,
        self_type,
        opts,
        ..
    } = arguments;
    let mut errors = vec![];
    let mut warnings = vec![];
    // ensure that the lhs is a variable expression or struct field access
    match lhs {
        ReassignmentTarget::VariableExpression(var) => {
            let mut expr = var;
            let mut names_vec = Vec::new();
            let (base_name, final_return_type) = loop {
                match *expr {
                    Expression::VariableExpression { name, .. } => {
                        // check that the reassigned name exists
                        let unknown_decl = check!(
                            namespace.resolve_symbol(&name).cloned(),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        let variable_decl = check!(
                            unknown_decl.expect_variable().cloned(),
                            return err(warnings, errors),
                            warnings,
                            errors
                        );
                        if !variable_decl.is_mutable.is_mutable() {
                            errors.push(CompileError::AssignmentToNonMutable { name });
                            return err(warnings, errors);
                        }
                        break (name, variable_decl.body.return_type);
                    }
                    Expression::SubfieldExpression {
                        prefix,
                        field_to_access,
                        ..
                    } => {
                        names_vec.push(ProjectionKind::StructField {
                            name: field_to_access,
                        });
                        expr = prefix;
                    }
                    Expression::TupleIndex {
                        prefix,
                        index,
                        index_span,
                        ..
                    } => {
                        names_vec.push(ProjectionKind::TupleField { index, index_span });
                        expr = prefix;
                    }
                    _ => {
                        errors.push(CompileError::InvalidExpressionOnLhs { span });
                        return err(warnings, errors);
                    }
                }
            };
            let names_vec = names_vec.into_iter().rev().collect::<Vec<_>>();
            let (ty_of_field, _ty_of_parent) = check!(
                namespace.find_subfield_type(&base_name, &names_vec),
                return err(warnings, errors),
                warnings,
                errors
            );
            // type check the reassignment
            let rhs = check!(
                TypedExpression::type_check(TypeCheckArguments {
                    checkee: rhs,
                    namespace,
                    return_type_annotation: ty_of_field,
                    help_text: Default::default(),
                    self_type,
                    mode: Mode::NonAbi,
                    opts,
                }),
                error_recovery_expr(span),
                warnings,
                errors
            );

            ok(
                TypedDeclaration::Reassignment(TypedReassignment {
                    lhs_base_name: base_name,
                    lhs_type: final_return_type,
                    lhs_indices: names_vec,
                    rhs,
                }),
                warnings,
                errors,
            )
        }
        ReassignmentTarget::StorageField(fields) => reassign_storage_subfield(TypeCheckArguments {
            checkee: (fields, span, rhs),
            namespace,
            return_type_annotation: insert_type(TypeInfo::Unknown),
            help_text: Default::default(),
            self_type,
            mode: Mode::NonAbi,
            opts,
        })
        .map(TypedDeclaration::StorageReassignment),
    }
}

/// Used to create a stubbed out function when the function fails to compile, preventing cascading
/// namespace errors
fn error_recovery_function_declaration(decl: FunctionDeclaration) -> TypedFunctionDeclaration {
    let FunctionDeclaration {
        name,
        return_type,
        span,
        return_type_span,
        visibility,
        ..
    } = decl;
    TypedFunctionDeclaration {
        purity: Default::default(),
        name,
        body: TypedCodeBlock {
            contents: Default::default(),
        },
        span,
        is_contract_call: false,
        return_type_span,
        parameters: Default::default(),
        visibility,
        return_type: insert_type(return_type),
        type_parameters: Default::default(),
    }
}

/// Describes each field being drilled down into in storage and its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeCheckedStorageReassignment {
    pub fields: Vec<TypeCheckedStorageReassignDescriptor>,
    pub(crate) ix: StateIndex,
    pub rhs: TypedExpression,
}

impl Spanned for TypeCheckedStorageReassignment {
    fn span(&self) -> Span {
        self.fields
            .iter()
            .fold(self.fields[0].span.clone(), |acc, field| {
                Span::join(acc, field.span.clone())
            })
    }
}

impl TypeCheckedStorageReassignment {
    pub fn names(&self) -> Vec<Ident> {
        self.fields
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>()
    }
}

/// Describes a single subfield access in the sequence when reassigning to a subfield within
/// storage.
#[derive(Clone, Debug, Eq)]
pub struct TypeCheckedStorageReassignDescriptor {
    pub name: Ident,
    pub r#type: TypeId,
    pub(crate) span: Span,
}

// NOTE: Hash and PartialEq must uphold the invariant:
// k1 == k2 -> hash(k1) == hash(k2)
// https://doc.rust-lang.org/std/collections/struct.HashMap.html
impl PartialEq for TypeCheckedStorageReassignDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && look_up_type_id(self.r#type) == look_up_type_id(other.r#type)
    }
}

fn reassign_storage_subfield(
    arguments: TypeCheckArguments<'_, (Vec<Ident>, Span, Expression)>,
) -> CompileResult<TypeCheckedStorageReassignment> {
    let TypeCheckArguments {
        checkee: (fields, span, rhs),
        namespace,
        return_type_annotation: _return_type_annotation,
        help_text: _help_text,
        self_type,
        opts,
        ..
    } = arguments;
    let mut errors = vec![];
    let mut warnings = vec![];
    if !namespace.has_storage_declared() {
        errors.push(CompileError::NoDeclaredStorage { span });

        return err(warnings, errors);
    }

    let storage_fields = check!(
        namespace.get_storage_field_descriptors(),
        return err(warnings, errors),
        warnings,
        errors
    );
    let mut type_checked_buf = vec![];
    let mut fields: Vec<_> = fields.into_iter().rev().collect();

    let first_field = fields.pop().expect("guaranteed by grammar");
    let (ix, initial_field_type) = match storage_fields
        .iter()
        .enumerate()
        .find(|(_, TypedStorageField { name, .. })| name == &first_field)
    {
        Some((ix, TypedStorageField { r#type, .. })) => (StateIndex::new(ix), r#type),
        None => {
            errors.push(CompileError::StorageFieldDoesNotExist {
                name: first_field.clone(),
            });
            return err(warnings, errors);
        }
    };

    type_checked_buf.push(TypeCheckedStorageReassignDescriptor {
        name: first_field.clone(),
        r#type: *initial_field_type,
        span: first_field.span(),
    });

    fn update_available_struct_fields(id: TypeId) -> Vec<TypedStructField> {
        match look_up_type_id(id) {
            TypeInfo::Struct { fields, .. } => fields,
            _ => vec![],
        }
    }
    let mut curr_type = *initial_field_type;

    // if the previously iterated type was a struct, put its fields here so we know that,
    // in the case of a subfield, we can type check the that the subfield exists and its type.
    let mut available_struct_fields = update_available_struct_fields(*initial_field_type);

    // get the initial field's type
    // make sure the next field exists in that type
    for field in fields.into_iter().rev() {
        match available_struct_fields
            .iter()
            .find(|x| x.name.as_str() == field.as_str())
        {
            Some(struct_field) => {
                curr_type = struct_field.r#type;
                type_checked_buf.push(TypeCheckedStorageReassignDescriptor {
                    name: field.clone(),
                    r#type: struct_field.r#type,
                    span: field.span().clone(),
                });
                available_struct_fields = update_available_struct_fields(struct_field.r#type);
            }
            None => {
                let available_fields = available_struct_fields
                    .iter()
                    .map(|x| x.name.as_str())
                    .collect::<Vec<_>>();
                errors.push(CompileError::FieldNotFound {
                    field_name: field.clone(),
                    available_fields: available_fields.join(", "),
                    struct_name: type_checked_buf.last().unwrap().name.clone(),
                });
                return err(warnings, errors);
            }
        }
    }
    let rhs = check!(
        TypedExpression::type_check(TypeCheckArguments {
            checkee: rhs,
            namespace,
            return_type_annotation: curr_type,
            help_text: Default::default(),
            self_type,
            mode: Mode::NonAbi,
            opts,
        }),
        error_recovery_expr(span),
        warnings,
        errors
    );

    ok(
        TypeCheckedStorageReassignment {
            fields: type_checked_buf,
            ix,
            rhs,
        },
        warnings,
        errors,
    )
}
