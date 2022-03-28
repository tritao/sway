use super::token_type::{get_trait_details, TokenType, VariableDetails};
use crate::{
    core::token_type::{get_function_details, get_struct_details},
    utils::common::extract_var_body,
};
use sway_core::{
    AstNode, AstNodeContent, Declaration, Expression, VariableDeclaration,
    type_engine::TypeInfo,
};
use sway_types::{ident::Ident, span::Span};
use tower_lsp::lsp_types::{Position, Range};

#[derive(Debug, Clone)]
pub struct Token {
    pub range: Range,
    pub token_type: TokenType,
    pub name: String,
    pub line_start: u32,
    pub length: u32,
}

impl Token {
    pub fn new(span: &Span, name: String, token_type: TokenType) -> Self {
        let range = get_range_from_span(span);

        Self {
            range,
            name,
            token_type,
            line_start: range.start.line,
            length: range.end.character - range.start.character + 1,
        }
    }

    pub fn is_within_character_range(&self, character: u32) -> bool {
        let range = self.range;
        character >= range.start.character && character <= range.end.character
    }

    pub fn is_same_type(&self, other_token: &Token) -> bool {
        if other_token.token_type == self.token_type {
            true
        } else {
            matches!(
                (&other_token.token_type, &self.token_type),
                (
                    TokenType::FunctionApplication,
                    TokenType::FunctionDeclaration(_)
                ) | (
                    TokenType::FunctionDeclaration(_),
                    TokenType::FunctionApplication
                ),
            )
        }
    }

    pub fn get_line_start(&self) -> u32 {
        self.line_start
    }

    pub fn from_variable(var_dec: &VariableDeclaration) -> Self {
        let ident = &var_dec.name;
        let name = ident.as_str();
        let var_body = extract_var_body(var_dec);

        Token::new(
            ident.span(),
            name.into(),
            TokenType::Variable(VariableDetails {
                is_mutable: var_dec.is_mutable,
                var_body,
            }),
        )
    }

    pub fn from_ident(ident: &Ident, token_type: TokenType) -> Self {
        Token::new(ident.span(), ident.as_str().into(), token_type)
    }

    pub fn from_span(span: Span, token_type: TokenType) -> Self {
        Token::new(&span, span.as_str().into(), token_type)
    }

    pub fn is_initial_declaration(&self) -> bool {
        !matches!(
            self.token_type,
            TokenType::Reassignment | TokenType::FunctionApplication
        )
    }
}

use forc_util::{println_red_err, println_yellow_err, println_green_err};


pub fn traverse_node(node: AstNode, tokens: &mut Vec<Token>) {
    match node.content {
        AstNodeContent::Declaration(dec) => handle_declaration(dec, tokens),
        AstNodeContent::Expression(exp) => handle_expression(exp, tokens),
        AstNodeContent::ImplicitReturnExpression(exp) => handle_expression(exp, tokens),
        // TODO
        // handle other content types
        AstNodeContent::UseStatement(_) => println_red_err("AstNodeContent::UseStatement").unwrap(),
        AstNodeContent::ReturnStatement(_) => println_red_err("AstNodeContent::ReturnStatement").unwrap(),
        AstNodeContent::WhileLoop(_) => println_red_err("AstNodeContent::WhileLoop").unwrap(),
        AstNodeContent::IncludeStatement(_) => println_red_err("AstNodeContent::IncludeStatement").unwrap(),
        _ => {}
    };
}

fn handle_declaration(declaration: Declaration, tokens: &mut Vec<Token>) {
    match declaration {
        Declaration::VariableDeclaration(variable) => {
            println_green_err(&format!("Declaration::VariableDeclaration: name: {}", variable.name.span().as_str())).unwrap();
            tokens.push(Token::from_variable(&variable));
            handle_expression(variable.body, tokens);
        }
        Declaration::FunctionDeclaration(func_dec) => {
            println_green_err(&format!("Declaration::FunctionDeclaration: name: {}()", func_dec.name.span().as_str())).unwrap();
            let ident = &func_dec.name;
            let token = Token::from_ident(
                ident,
                TokenType::FunctionDeclaration(get_function_details(&func_dec)),
            );
            tokens.push(token);

            for node in func_dec.body.contents {
                traverse_node(node, tokens);
            }
        }
        Declaration::Reassignment(reassignment) => {
            let token_type = TokenType::Reassignment;
            let token = Token::from_span(reassignment.lhs_span(), token_type);
            println_green_err(&format!("Declaration::Reassignment: name: {}", &token.name)).unwrap();
            tokens.push(token);
            handle_expression(reassignment.rhs, tokens);
        }

        Declaration::TraitDeclaration(trait_dec) => {
            let ident = &trait_dec.name;
            let token = Token::from_ident(ident, TokenType::Trait(get_trait_details(&trait_dec)));
            println_green_err(&format!("Declaration::TraitDeclaration: name: {}", &token.name)).unwrap();

            tokens.push(token);

            // todo
            // traverse methods: Vec<FunctionDeclaration<'sc>> field as well ?
        }
        Declaration::StructDeclaration(struct_dec) => {
            println_green_err(&format!("Declaration::StructDeclaration: name: {}", &struct_dec.name.span().as_str())).unwrap();

            let ident = &struct_dec.name;
            let token =
                Token::from_ident(ident, TokenType::Struct(get_struct_details(&struct_dec)));
            tokens.push(token);
        }
        Declaration::EnumDeclaration(enum_dec) => {
            println_green_err(&format!("Declaration::EnumDeclaration: name: {}", &enum_dec.name.span().as_str())).unwrap();

            let ident = enum_dec.name;
            let token = Token::from_ident(&ident, TokenType::Enum);
            tokens.push(token);
        }

        Declaration::ImplTrait(_) => println_red_err("Declaration::ImplTrait").unwrap(),
        Declaration::ImplSelf(_impl_self) => {
            println_red_err("Declaration::ImplSelf").unwrap();
            //println_yellow_err(&format!("type_implementing_for {:#?}", impl_self.type_implementing_for)).unwrap();
            //println_yellow_err(&format!("type_arguments {:#?}", impl_self.type_arguments)).unwrap();
            //println_yellow_err(&format!("functions {:#?}", impl_self.functions)).unwrap();
            // if let TypeInfo::Custom { name } = impl_self.type_implementing_for {
            //     let token = Token::from_ident(&name, TokenType::Struct);
            //     tokens.push(token);
            // }
        },
        Declaration::AbiDeclaration(_) => println_red_err("Declaration::AbiDeclaration").unwrap(),
        Declaration::ConstantDeclaration(_) => println_red_err("Declaration::ConstantDeclaration").unwrap(),
        Declaration::StorageDeclaration(_) => println_red_err("Declaration::StorageDeclaration").unwrap(),

        _ => {}
    };
}

fn handle_expression(exp: Expression, tokens: &mut Vec<Token>) {
    match exp {
        Expression::CodeBlock { span: _, contents } => {
            println_green_err(&format!("Expression::CodeBlock")).unwrap();

            let nodes = contents.contents;

            for node in nodes {
                traverse_node(node, tokens);
            }
        }
        Expression::FunctionApplication { name, .. } => {
            let ident = name.suffix;
            let token = Token::from_ident(&ident, TokenType::FunctionApplication);
            println_green_err(&format!("Expression::FunctionApplication: name: {}", &token.name)).unwrap();

            tokens.push(token);

            // TODO
            // perform a for/in on arguments ?
        },
        // TODO
        // handle other expressions

        Expression::Literal{..} => println_red_err("Expression::Literal").unwrap(),
        Expression::LazyOperator{..} => println_red_err("Expression::LazyOperator").unwrap(),
        Expression::VariableExpression{..} => println_red_err("Expression::VariableExpression").unwrap(),
        Expression::Tuple{..} => println_red_err("Expression::Tuple").unwrap(),
        Expression::TupleIndex{..} => println_red_err("Expression::TupleIndex").unwrap(),
        Expression::StructExpression{..} => println_red_err("Expression::StructExpression").unwrap(),
        Expression::IfExp{..} => println_red_err("Expression::IfExp").unwrap(),
        Expression::MatchExp{..} => println_red_err("Expression::MatchExp").unwrap(),
        Expression::AsmExpression{..} => println_red_err("Expression::AsmExpression").unwrap(),
        Expression::MethodApplication{method_name, contract_call_params, arguments, ..} => {
            println_red_err("Expression::MethodApplication").unwrap();
            //println_yellow_err(&format!("---method_name {:#?}", method_name)).unwrap();
            //println_yellow_err(&format!("---contract_call_params {:#?}", contract_call_params)).unwrap();
            //println_yellow_err(&format!("---arguments {:#?}", arguments)).unwrap();
        },
        Expression::SubfieldExpression{..} => println_red_err("Expression::SubfieldExpression").unwrap(),
        Expression::DelineatedPath{..} => println_red_err("Expression::DelineatedPath").unwrap(),
        Expression::AbiCast{..} => println_red_err("Expression::AbiCast").unwrap(),
        Expression::ArrayIndex{..} => println_red_err("Expression::ArrayIndex").unwrap(),
        Expression::DelayedMatchTypeResolution{..} => println_red_err("Expression::DelayedMatchTypeResolution").unwrap(),
        Expression::IfLet{..} => println_red_err("Expression::IfLet").unwrap(),
        Expression::SizeOfVal{..} => println_red_err("Expression::SizeOfVal").unwrap(),
        Expression::SizeOfType{..} => println_red_err("Expression::SizeOfType").unwrap(),
        _ => {}
    }
}

fn get_range_from_span(span: &Span) -> Range {
    let start = span.start_pos().line_col();
    let end = span.end_pos().line_col();

    let start_line = start.0 as u32 - 1;
    let start_character = start.1 as u32 - 1;

    let end_line = end.0 as u32 - 1;
    let end_character = end.1 as u32 - 1;

    Range {
        start: Position::new(start_line, start_character),
        end: Position::new(end_line, end_character),
    }
}
