use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use lever_core::Translator;

#[proc_macro]
pub fn my_macro(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as syn::LitStr);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        // ...
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

