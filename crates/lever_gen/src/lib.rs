use lever_core::Child;
use lever_core::DirectOrRule;
use lever_core::Rule;
use lever_core::TreesitterNodeQuery;
use quote::format_ident;
use quote::quote;
use std::fs;
use syn::parse_macro_input;

use lever_core::LanguageDefinition;

#[proc_macro]
pub fn rules_translator(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::LitStr);

    let file_path = input.value();
    let file_contents = fs::read_to_string(file_path).expect("Could not read file");

    let language_def: LanguageDefinition = ron::from_str(&file_contents).unwrap();

    let mut rule_parsers = vec![];
    for rule in language_def.ast_rules {
        rule_parsers.push(gen_parse_rule(&rule));
    }

    let output = quote! {
        {
            fn child_by_kind<'a>(
                node: &'a tree_sitter::Node,
                kind: &'a str,
            ) -> Option<tree_sitter::Node<'a>> {
                let mut cursor = node.walk();

                for child in node.children(&mut cursor) {
                    if child.kind() == kind {
                        return Some(child);
                    }
                }

                None
            }

            struct GeneratedRuleTranslator{
                arena: indextree::Arena<Node>,
                source_code: String,
            };

            impl Translator for GeneratedRuleTranslator {
                fn translate(&mut self, source_code: &str, syntax_tree: tree_sitter::Tree) -> Ast {
                    let root_id = self.parse_Root(&syntax_tree.root_node()).unwrap();
                    self.source_code = source_code.to_string();
                    Ast::initialize(self.arena.clone(), root_id) // TODO: Remove clone?
                }
            }

            impl GeneratedRuleTranslator {

                fn new_node(
                    &mut self,
                    kind: &str,
                    syntax_node: &tree_sitter::Node,
                    symbol: Symbol,
                    import: Import,
                    semantic_token_type: Option<HighlightType>,
                ) -> NodeId {
                    self.arena.new_node(Node::new(
                        NodeKind::Node(kind.to_string()),
                        syntax_node,
                        &self.source_code,
                        symbol,
                        import,
                        semantic_token_type,
                    ))
                }

                fn new_error_node(
                    &mut self,
                    syntax_node: &tree_sitter::Node,
                    message: Option<String>,
                ) -> NodeId {
                    self.arena.new_node(Node::new(
                        NodeKind::Error(message),
                        syntax_node,
                        &self.source_code,
                        Symbol::None,
                        Import::None,
                        None,
                    ))
                }

                #(#rule_parsers)*
            }

            GeneratedRuleTranslator {
                arena: indextree::Arena::new(),
                source_code: String::new()
            }
        }
    };

    output.into()
}

fn gen_parse_rule(rule: &Rule) -> proc_macro2::TokenStream {
    let fn_name = format_ident!("parse_{}", &rule.node_name);
    let kind = rule.node_name.clone();

    let children = rule
        .children
        .iter()
        .map(gen_child)
        .collect::<Vec<proc_macro2::TokenStream>>();

    quote!(
        fn #fn_name (&mut self, node: &tree_sitter::Node) -> Option<indextree::NodeId> {
            let node_id = self.new_node(#kind, node, Symbol::None, Import::None, None); // TODO:
                                                                                           // Remove Nones

            #(#children)*

            Some(node_id)
        }
    )
}

fn gen_child(child: &Child) -> proc_macro2::TokenStream {
    let query = gen_query(&child.query);

    match &child.rule {
        DirectOrRule::Direct(name) => {
            let highlight_type = if let Some(ht) = &child.highlight_type {
                syn::parse_str(&format!("Some(HighlightType::{:?})", ht)).unwrap()
            } else {
                quote!(None)
            };

            quote!(
                if let Some(ts_node) = #query {
                    if ts_node.has_error() {
                        node_id
                            .append(self.new_error_node(&ts_node, None), &mut self.arena);
                    }

                    node_id.append(
                        self.new_node(
                            #name,
                            &node,
                            Symbol::None,
                            Import::None,
                            #highlight_type,
                        ),
                        &mut self.arena,
                    );
                }
            )
        }
        DirectOrRule::Rule(name) => {
            let fn_name = format_ident!("parse_{}", name);

            quote!(
                if let Some(node) = #query {
                    node_id.append(self.#fn_name(&node)?, &mut self.arena);
                }
            )
        }
    }
}

fn gen_query(query: &TreesitterNodeQuery) -> proc_macro2::TokenStream {
    match query {
        TreesitterNodeQuery::Path(path) => {
            if path.is_empty() {
                panic!("Empty paths are not allowed.");
            }

            let mut parts = vec![];
            parts.push(match path.first().unwrap() {
                TreesitterNodeQuery::Path(_) => {
                    unimplemented!("Nested paths are not supported.")
                }
                TreesitterNodeQuery::Kind(kind) => quote!(child_by_kind(node, #kind)),
                TreesitterNodeQuery::Field(field) => quote!(node.child_by_field_name(#field)),
            });

            for query in path.iter().skip(1) {
                parts.push(match query {
                    TreesitterNodeQuery::Path(_) => {
                        unimplemented!("Nested paths are not supported.")
                    }
                    TreesitterNodeQuery::Kind(kind) => quote!(
                        .and_then(|n| child_by_kind(node, #kind))
                    ),
                    TreesitterNodeQuery::Field(field) => quote!(
                        .and_then(|n| n.child_by_field_name(#field))
                    ),
                });
            }

            quote!(#(#parts)*)
        }
        TreesitterNodeQuery::Kind(kind) => quote!(child_by_kind(node, #kind)),
        TreesitterNodeQuery::Field(field) => quote!(node.child_by_field_name(#field)),
    }
}
