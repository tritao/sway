use crate::core::token::Token;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Hover, HoverContents, MarkupContent, MarkupKind};

// Flags for debugging various parts of the server
#[derive(Debug, Default)]
pub struct DebugFlags {
    /// Instructs the client to draw squiggly lines
    /// under all of the tokens that our server managed to parse
    pub parsed_tokens_as_warnings: bool,
    /// Display the token information in the hover tooltip
    pub token_info_on_hover: bool,
}

pub fn generate_warnings_for_parsed_tokens(tokens: &[Token]) -> Vec<Diagnostic> {
    let warnings = tokens
        .iter()
        .map(|token| Diagnostic {
            range: token.range,
            severity: Some(DiagnosticSeverity::WARNING),
            message: token.name.clone(),
            ..Default::default()
        })
        .collect();

    warnings
}

pub fn format_token_for_hover_tooltip(token: &Token, token_value: &String) -> Hover {
    let token_info = format!(
        "Token: {}\nTokenType: {:#?}\nRange: {:?}\nLine: {}\nColumn: {}\nLength: {}",
        token.name,
        token.token_type,
        token.range,
        token.range.start.line,
        token.range.start.character,
        token.length
    );

    Hover {
        contents: HoverContents::Markup(MarkupContent {
            value: format!("```sway\n{}\n```\n###Token Info\n{}", token_value, token_info),
            kind: MarkupKind::Markdown,
        }),
        range: Some(token.range),
    }
}