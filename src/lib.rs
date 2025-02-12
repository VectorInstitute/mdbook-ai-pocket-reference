//! # `mdbook-ai-pocket-reference`
//!
//! This crate produces a preprocessor for the [rust-lang mdbook](https://github.com/rust-lang/mdBook)
//! project that comprises of various link helpers for the [AI-Pocket-Reference](https://github.com/VectorInstitute/ai-pocket-reference)
//! project.
//!
//! ## Basic Usage
//!
//! First, install the crate:
//!
//! ```sh
//! cargo install mdbook-ai-pocket-reference
//! ```
//!
//! Next, and as with all preprocessor extensions, to include ` mdbook-ai-pocket-reference`
//! in your book, add the following to your `book.toml`:
//!
//! ```sh
//! [preprocessor.ai-pocket-reference]
//! command = " mdbook-ai-pocket-reference"
//! ```
//!
//! In order to add an author or list of authors in your chapter, there is currently
//! one supported helper:
//!
//! ```markdown
//! <!-- for including ai-pocket-reference header (default) -->
//! {{#aipr_header}}
//!
//! <!-- for including ai-pocket-reference header with colab link -->
//! {{ #aipr_header colab=nlp/lora.ipynb}}
//!
//! ```
//!
//! For more details see the project's [README](https://github.com/VectorInstitute/mdbook-ai-pocket-reference)

pub mod ai_pocket_reference;

pub use ai_pocket_reference::AIPRPreprocessor;
