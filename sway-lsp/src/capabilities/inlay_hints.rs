use tower_lsp::lsp_types::{
    self, Range, InlayHintParams,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlayHintsConfig {
    /// Whether to render leading colons for type hints, and trailing colons for parameter hints.
    pub render_colons: bool,
    /// Whether to show inlay type hints for variables.
    pub type_hints: bool,
    /// Whether to show function parameter name inlay hints at the call site.
    pub parameter_hints: bool,
    /// Whether to show inlay type hints for method chains.
    pub chaining_hints: bool,
    /// Maximum length for inlay hints. Set to null to have an unlimited length.
    pub max_length: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlayKind {
    TypeHint,
    ParameterHint,
    ChainingHint,
    GenericParamListHint,
}

#[derive(Debug)]
pub struct InlayHint {
    pub range: Range,
    pub kind: InlayKind,
    pub label: String,
}

pub(crate) fn inlay_hints(params: &InlayHintParams, config: &InlayHintsConfig) {

}

pub(crate) fn inlay_hint(
    render_colons: bool,
    inlay_hint: InlayHint,
) -> lsp_types::InlayHint {
    lsp_types::InlayHint {
        position: match inlay_hint.kind {
            // before annotated thing
            InlayKind::ParameterHint => inlay_hint.range.start,
            // after annotated thing
            InlayKind::TypeHint
            | InlayKind::ChainingHint
            | InlayKind::GenericParamListHint => inlay_hint.range.end,
        },
        label: lsp_types::InlayHintLabel::String(match inlay_hint.kind {
            InlayKind::ParameterHint if render_colons => format!("{}:", inlay_hint.label),
            InlayKind::TypeHint if render_colons => format!(": {}", inlay_hint.label),
            _ => inlay_hint.label.to_string(),
        }),
        kind: match inlay_hint.kind {
            InlayKind::ParameterHint => Some(lsp_types::InlayHintKind::PARAMETER),
            InlayKind::TypeHint | InlayKind::ChainingHint => {
                Some(lsp_types::InlayHintKind::TYPE)
            }
            InlayKind::GenericParamListHint => None,
        },
        tooltip: None,
        padding_left: Some(match inlay_hint.kind {
            InlayKind::TypeHint => !render_colons,
            InlayKind::ParameterHint => false,
            InlayKind::ChainingHint => true,
            InlayKind::GenericParamListHint => false,
        }),
        padding_right: Some(match inlay_hint.kind {
            InlayKind::TypeHint | InlayKind::ChainingHint => {
                false
            }
            InlayKind::ParameterHint => true,
            InlayKind::GenericParamListHint => false,
        }),
        text_edits: None,
        data: None,
    }
}

impl Default for InlayHintsConfig {
    fn default() -> Self {
        Self {
            render_colons: true,
            type_hints: true,
            parameter_hints: true,
            chaining_hints: true,
            max_length: Some(25),
        }
    }
}