# Lever Framework

Lever is an open-source framework designed to simplify the creation of editor support for Domain-Specific Languages (DSLs).
Lever leverages existing language artifacts, such as the grammar and tooling, and integrates with the Language Server Protocol (LSP) to provide syntax highlighting, code completion, and other essential editing features for DSLs.
This framework aims to lower the barrier to creating rich editing environments for DSLs without the complexities of building tooling from scratch.

## Features

- **Cross-Editor Support**: Utilizes the LSP to enable editor support across various code editors, including VS Code, Sublime Text, Vim, and others.
- **Integration of Existing Tooling**: Lever supports the integration of existing DSL tooling, such as compilers and static analyzers, directly into the editor environment through adapters. 
- **Syntax Highlighting**: Semantic tokens for context-aware syntax highlighting.
- **Auto-Completion**: Code suggestions based on keywords and language symbols.
- **Go to Definition**: Allows users to navigate to symbol definitions within the DSL.
- **Renaming**: Enables symbol renaming with automatic propagation across the codebase.
- **Error Diagnostics**: Real-time syntax checking.

## Rule Language

Lever uses a lightweight rule-based language to define language-specific details.
The rules define mappings from the DSL’s **Tree-Sitter** grammar to the abstract syntax tree (AST) and symbol table (ST), providing a consistent internal representation for editor features.

Consider the following DSL syntax from [Protobuf](https://protobuf.dev/):

```proto
message Person {  
    int32 id = 2;    
    string name = 1;  
    string email = 3;
}
```

To parse this, Lever leverages an existing Tree-Sitter grammar, which could look like:

```js
message: $ => seq(
    'message',
    $.message_name,
    $.message_body,
),

message_body: $ => seq(
    '{',
    repeat(choice(
        $.field,
        $.enum,
        $.message,
        $.option,
        $.oneof,
        $.map_field,
        $.reserved,
        $.empty_statement,
    )),
    '}',
),
```

Lever's rule language then specifies how to map the syntax tree nodes to an AST and ST for use in editor features:


```ron
Rule(
    node_name: "Message",
    is_scope: true,
    symbol: Init(type: "Message", name_node: "Name"),
    children: [
        (query: Kind("message_name"), rule: Direct("Name")),
        (query: Kind("message_body"), rule: Rule("MessageBody")),
    ]
)
```

This rule:

- **Scope**: Defines `Message` as a scoped node, creating a distinct region in the AST where identifiers and symbols within the message body are isolated from other scopes.
- **Symbol**: Initializes `Message` as a symbol of type `Message` with its identifier derived from the `Name` node, allowing Lever to recognize it as a unique symbol within the AST.
- **Children**: Specifies `message_name` as a `Name` node and delegates the handling of the `message_body` to the `MessageBody` rule.

Lever's rule language allows DSL creators to add necessary semantics over the existing syntax, enabling rich editor support while staying lightweight.

## Getting Started

To create a new Lever project:

1. Run the setup script using [Cookiecutter](https://github.com/cookiecutter/cookiecutter), which will scaffold a project with the basic components, including a sample LSP server and VS Code extension.

```bash
cookiecutter https://github.com/ace-design/lever-framework-cutter
```

2. Define language specifics using the Lever rule language (see below).
3. Use cargo run to build the language server and start developing editor support for your DSL.
