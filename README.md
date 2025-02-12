# mdbook-ai-pocket-reference

---------------------------------------------------------------------------------------

[![Lint](https://github.com/VectorInstitute/mdbook-ai-pocket-reference/actions/workflows/lint.yml/badge.svg)](https://github.com/VectorInstitute/mdbook-ai-pocket-reference/actions/workflows/lint.yml)
[![Test Docs](https://github.com/VectorInstitute/mdbook-ai-pocket-reference/actions/workflows/test_docs.yml/badge.svg)](https://github.com/VectorInstitute/mdbook-ai-pocket-reference/actions/workflows/test_docs.yml)
[![Test Lib](https://github.com/VectorInstitute/mdbook-ai-pocket-reference/actions/workflows/test.yml/badge.svg)](https://github.com/VectorInstitute/mdbook-ai-pocket-reference/actions/workflows/test.yml)
![GitHub License](https://img.shields.io/github/license/VectorInstitute/mdbook-ai-pocket-reference)
![GitHub Release](https://img.shields.io/github/v/release/VectorInstitute/mdbook-ai-pocket-reference)
![docs.rs](https://img.shields.io/docsrs/mdbook-ai-pocket-reference)

A preprocessor for [mdbook](https://rust-lang.github.io/mdBook/) that creates
chapter-level contributor sections featuring authors' GitHub profiles.

## Installation

```bash
cargo install mdbook-ai-pocket-reference
```

## Usage

1. Add to your `book.toml`:

```toml
[preprocessor.ai-pocket-reference]
command = "mdbook-ai-pocket-reference"
```

1. Add ai-pocket-reference header:

```markdown
# Chapter Title

<!-- Default header -->
{{#aipr_header}}

<!-- Default header with colab -->
{{#aipr_header colab=nlp/lora.ipynb}}

<!-- Default header with colab and no reading time -->
{{#aipr_header colab=nlp/lora.ipynb,reading_time=false}}
```

The preprocessor will expand the helper to include the established header style
for AI Pocket References.

## Examples

```markdown
# LoRA

{{#aipr_header colab=nlp/lora.ipynb}}

Low-rank adaptation (LoRA) is parameter-efficient fine-tuning (PEFT) introduced
by Hu, Edward J. et...

```

Will render as:

<img width="846" alt="image" src="https://github.com/user-attachments/assets/a6812900-4f7f-4cc8-b0d4-1e4a67a558c0" />

