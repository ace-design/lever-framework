pub struct Setup {
    pub language_def: String,
    pub treesitter_language: tree_sitter::Language,
    pub translator: dyn crate::Translator,
}
