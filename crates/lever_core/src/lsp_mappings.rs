use serde::Deserialize;
use tower_lsp::lsp_types::{self, CompletionItemKind};

#[derive(Debug, Deserialize, Clone)]
pub enum SymbolCompletionType {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl SymbolCompletionType {
    pub const fn get(&self) -> CompletionItemKind {
        match self {
            Self::Text => CompletionItemKind::TEXT,
            Self::Method => CompletionItemKind::METHOD,
            Self::Function => CompletionItemKind::FUNCTION,
            Self::Constructor => CompletionItemKind::CONSTRUCTOR,
            Self::Field => CompletionItemKind::FIELD,
            Self::Variable => CompletionItemKind::VARIABLE,
            Self::Class => CompletionItemKind::CLASS,
            Self::Interface => CompletionItemKind::INTERFACE,
            Self::Module => CompletionItemKind::MODULE,
            Self::Property => CompletionItemKind::PROPERTY,
            Self::Unit => CompletionItemKind::UNIT,
            Self::Value => CompletionItemKind::VALUE,
            Self::Enum => CompletionItemKind::ENUM,
            Self::Keyword => CompletionItemKind::KEYWORD,
            Self::Snippet => CompletionItemKind::SNIPPET,
            Self::Color => CompletionItemKind::COLOR,
            Self::File => CompletionItemKind::FILE,
            Self::Reference => CompletionItemKind::REFERENCE,
            Self::Folder => CompletionItemKind::FOLDER,
            Self::EnumMember => CompletionItemKind::ENUM_MEMBER,
            Self::Constant => CompletionItemKind::CONSTANT,
            Self::Struct => CompletionItemKind::STRUCT,
            Self::Event => CompletionItemKind::EVENT,
            Self::Operator => CompletionItemKind::OPERATOR,
            Self::TypeParameter => CompletionItemKind::TYPE_PARAMETER,
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum HighlightType {
    Namespace,
    Type,
    Class,
    Enum,
    Interface,
    Struct,
    TypeParameter,
    Parameter,
    Variable,
    Property,
    EnumMember,
    Event,
    Function,
    Method,
    Macro,
    Keyword,
    Modifier,
    Comment,
    String,
    Number,
    Regexp,
    Operator,
    Decorator,
}

impl HighlightType {
    pub const fn get(&self) -> lsp_types::SemanticTokenType {
        match self {
            Self::Namespace => lsp_types::SemanticTokenType::NAMESPACE,
            Self::Type => lsp_types::SemanticTokenType::TYPE,
            Self::Class => lsp_types::SemanticTokenType::CLASS,
            Self::Enum => lsp_types::SemanticTokenType::ENUM,
            Self::Interface => lsp_types::SemanticTokenType::INTERFACE,
            Self::Struct => lsp_types::SemanticTokenType::STRUCT,
            Self::TypeParameter => lsp_types::SemanticTokenType::TYPE_PARAMETER,
            Self::Parameter => lsp_types::SemanticTokenType::PARAMETER,
            Self::Variable => lsp_types::SemanticTokenType::VARIABLE,
            Self::Property => lsp_types::SemanticTokenType::PROPERTY,
            Self::EnumMember => lsp_types::SemanticTokenType::ENUM_MEMBER,
            Self::Event => lsp_types::SemanticTokenType::EVENT,
            Self::Function => lsp_types::SemanticTokenType::FUNCTION,
            Self::Method => lsp_types::SemanticTokenType::METHOD,
            Self::Macro => lsp_types::SemanticTokenType::MACRO,
            Self::Keyword => lsp_types::SemanticTokenType::KEYWORD,
            Self::Modifier => lsp_types::SemanticTokenType::MODIFIER,
            Self::Comment => lsp_types::SemanticTokenType::COMMENT,
            Self::String => lsp_types::SemanticTokenType::STRING,
            Self::Number => lsp_types::SemanticTokenType::NUMBER,
            Self::Regexp => lsp_types::SemanticTokenType::REGEXP,
            Self::Operator => lsp_types::SemanticTokenType::OPERATOR,
            Self::Decorator => lsp_types::SemanticTokenType::DECORATOR,
        }
    }
}
