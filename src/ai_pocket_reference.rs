use handlebars::{to_json, Handlebars};
use mdbook::book::{Book, BookItem};
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use once_cell::sync::Lazy;
use regex::{CaptureMatches, Captures, Regex};
use serde::Serialize;
use serde_json::value::Map;
use std::collections::HashMap;

const AIPR_HEADER_TEMPLATE: &str = include_str!("./templates/header.hbs");
const AIPR_FOOTER_HTML: &str = include_str!("./templates/footer.html");
const MDLINK_TEMPLATE: &str = include_str!("./templates/md_link.hbs");
const WORDS_PER_MINUTE: usize = 200;

#[derive(Default)]
pub struct AIPRPreprocessor;

/// A preprocessor for expanding AI-Pocket-Reference helpers.
///
/// Supported helpers are:
///
/// - `{{#aipr_header <param-str>}}` - Adds the ai-pocket-reference header (optional param-str)
impl AIPRPreprocessor {
    pub(crate) const NAME: &'static str = "ai-pocket-reference";

    /// Create a new `AIPRPreprocessor`.
    pub fn new() -> Self {
        AIPRPreprocessor
    }
}

impl Preprocessor for AIPRPreprocessor {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> anyhow::Result<Book> {
        // This run method's implementation follows the implementation of
        // mdbook::preprocess::links::LinkPreprocessor.run().
        book.for_each_mut(|section: &mut BookItem| {
            if let BookItem::Chapter(ref mut ch) = *section {
                let word_count = words_count::count(&ch.content);
                let mut content = replace_all(&ch.content, word_count.words);

                // add footer with logo
                content.push_str(AIPR_FOOTER_HTML);

                // mutate chapter content
                ch.content = content;
            }
        });
        Ok(book)
    }
}

fn replace_all(s: &str, num_words: usize) -> String {
    // First replace all AIPR links
    let aipr_replaced = replace_all_aipr_links(s, num_words);

    // Then replace all Markdown links
    replace_all_md_links(&aipr_replaced)
}

fn replace_all_aipr_links(s: &str, num_words: usize) -> String {
    // This implementation follows closely to the implementation of
    // mdbook::preprocess::links::replace_all.
    let mut previous_end_index = 0;
    let mut replaced = String::new();

    for link in find_aipr_links(s) {
        replaced.push_str(&s[previous_end_index..link.start_index]);
        let new_content = link.render(num_words).unwrap(); // todo: better error handling
        replaced.push_str(&new_content);
        previous_end_index = link.end_index;
    }

    replaced.push_str(&s[previous_end_index..]);
    replaced
}

fn replace_all_md_links(s: &str) -> String {
    let mut previous_end_index = 0;
    let mut replaced = String::new();

    for link in find_md_links(s) {
        // Add text up to the current link
        let prefix = &s[previous_end_index..link.start_index];
        replaced.push_str(prefix);

        // Check if the prefix ends with a backslash or exclamation mark
        let last_char = prefix.chars().last();
        let is_escaped = last_char == Some('\\') || last_char == Some('!');

        if is_escaped {
            // For escaped links, just add the original link text
            replaced.push_str(&s[link.start_index..link.end_index]);
        } else {
            // For normal links, render as HTML
            let new_content = link.render().unwrap();
            replaced.push_str(&new_content);
        }

        previous_end_index = link.end_index;
    }

    replaced.push_str(&s[previous_end_index..]);
    replaced
}

#[derive(PartialEq, Debug, Clone)]
enum AIPRLinkType {
    Header(AIPRHeaderSettings),
}

#[derive(Debug, Clone, PartialEq)]
struct AIPRHeaderSettings {
    reading_time: bool,
    submit_issue: bool,
    colab: Option<String>,
}

impl Default for AIPRHeaderSettings {
    fn default() -> Self {
        Self {
            reading_time: true,
            submit_issue: true,
            colab: None,
        }
    }
}

fn _parse_param_str(param_str: &str) -> HashMap<String, String> {
    param_str
        .split(',')
        .filter_map(|pair| {
            pair.split_once('=')
                .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

impl AIPRHeaderSettings {
    fn from_param_str(param_str: &str) -> Self {
        let param_map = _parse_param_str(param_str);
        let colab = param_map.get("colab").map(|s| s.to_owned());
        let reading_time =
            !matches!(param_map.get("reading_time"), Some(bool_str) if (bool_str == "false"));
        let submit_issue =
            !matches!(param_map.get("submit_issue"), Some(bool_str) if (bool_str == "false"));

        Self {
            reading_time,
            submit_issue,
            colab,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
struct AIPRLink<'a> {
    start_index: usize,
    end_index: usize,
    link_type: AIPRLinkType,
    link_text: &'a str,
}

impl<'a> AIPRLink<'a> {
    #[allow(dead_code)]
    fn from_capture(cap: Captures<'a>) -> Option<AIPRLink<'a>> {
        let link_type = match (cap.get(0), cap.get(1), cap.get(2)) {
            (_, Some(typ), None) if typ.as_str() == "aipr_header" => {
                Some(AIPRLinkType::Header(AIPRHeaderSettings::default()))
            }
            (_, Some(typ), Some(param_str)) if typ.as_str() == "aipr_header" => {
                Some(AIPRLinkType::Header(AIPRHeaderSettings::from_param_str(
                    param_str.as_str().trim(),
                )))
            }
            _ => None,
        };

        link_type.and_then(|lnk_type| {
            cap.get(0).map(|mat| AIPRLink {
                start_index: mat.start(),
                end_index: mat.end(),
                link_type: lnk_type,
                link_text: mat.as_str(),
            })
        })
    }

    fn render(&self, num_words: usize) -> anyhow::Result<String> {
        match &self.link_type {
            AIPRLinkType::Header(settings) => {
                let mut handlebars = Handlebars::new();
                // register template from const str and assign a name to it
                handlebars
                    .register_template_string("aipr_header", AIPR_HEADER_TEMPLATE)
                    .unwrap();

                // create data for rendering handlebar
                let mut data = Map::new();
                if let Some(colab_path) = &settings.colab {
                    let colab_nb = ColabNB {
                        path: colab_path.to_owned(),
                    };
                    data.insert("colab_nb".to_string(), to_json(colab_nb));
                }
                data.insert("submit_issue".to_string(), to_json(settings.submit_issue));
                if settings.reading_time {
                    let rt_in_mins = (num_words as f32 / WORDS_PER_MINUTE as f32).round();
                    let rt = ReadingTime {
                        value: format!("{:.0} min", rt_in_mins),
                    };
                    data.insert("reading_time".to_string(), to_json(rt));
                }

                // render
                let html_string = handlebars.render("aipr_header", &data)?;

                Ok(html_string)
            }
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct ColabNB {
    path: String,
}

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct ReadingTime {
    value: String,
}

struct AIPRLinkIter<'a>(CaptureMatches<'a, 'a>);

impl<'a> Iterator for AIPRLinkIter<'a> {
    type Item = AIPRLink<'a>;
    fn next(&mut self) -> Option<AIPRLink<'a>> {
        for cap in &mut self.0 {
            if let Some(inc) = AIPRLink::from_capture(cap) {
                return Some(inc);
            }
        }
        None
    }
}

fn find_aipr_links(contents: &str) -> AIPRLinkIter<'_> {
    // lazily compute following regex
    // r"\\\{\{#.*\}\}|\{\{#([a-zA-Z0-9]+)\s*([^}]+)\}\}")?;
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)              # insignificant whitespace mode
        \\\{\{\#.*\}\}      # match escaped link
        |                   # or
        \{\{\s*             # link opening parens and whitespace
        \#([a-zA-Z0-9_]+)   # link type
        \s+                 # separating whitespace
        ([^}]+)?            # link target path and space separated properties (optional)
        \}\}                # link closing parens",
        )
        .unwrap()
    });

    AIPRLinkIter(RE.captures_iter(contents))
}

#[derive(PartialEq, Debug, Clone)]
struct MDLink<'a> {
    start_index: usize,
    end_index: usize,
    text: &'a str,
    url: &'a str,
}

impl<'a> MDLink<'a> {
    #[allow(dead_code)]
    fn from_capture(cap: Captures<'a>) -> Option<MDLink<'a>> {
        let md_tuple = match (cap.get(0), cap.get(1), cap.get(2)) {
            (_, Some(text_str), Some(url_str))
                if (url_str.as_str().starts_with("https://")
                    || url_str.as_str().starts_with("http://")) =>
            {
                Some((text_str.as_str(), url_str.as_str()))
            }
            _ => None,
        };

        md_tuple.and_then(|(text, url)| {
            cap.get(0).map(|mat| MDLink {
                start_index: mat.start(),
                end_index: mat.end(),
                text,
                url,
            })
        })
    }

    #[allow(dead_code)]
    fn render(&self) -> anyhow::Result<String> {
        let mut handlebars = Handlebars::new();

        // register template
        handlebars
            .register_template_string("md_link_expansion", MDLINK_TEMPLATE)
            .unwrap();

        // create data for rendering handlebar
        let mut data = Map::new();
        data.insert("text".to_string(), to_json(self.text));
        data.insert("url".to_string(), to_json(self.url));

        // render
        let html_string = handlebars.render("md_link_expansion", &data)?;

        Ok(html_string)
    }
}

struct MDLinkIter<'a>(CaptureMatches<'a, 'a>);

impl<'a> Iterator for MDLinkIter<'a> {
    type Item = MDLink<'a>;
    fn next(&mut self) -> Option<MDLink<'a>> {
        for cap in &mut self.0 {
            if let Some(inc) = MDLink::from_capture(cap) {
                return Some(inc);
            }
        }
        None
    }
}

fn find_md_links(contents: &str) -> MDLinkIter<'_> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)
            \[([^\]]*(?:\\.[^\]]*)*)\]    # link text in square brackets
            \(([^)]*(?:\\.[^)]*)*)\)      # link URL in parentheses
            ",
        )
        .unwrap()
    });

    MDLinkIter(RE.captures_iter(contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rstest::*;

    #[fixture]
    fn simple_book_content() -> String {
        "{{ #aipr_header }} {{ #aipr_header colab=nlp/lora.ipynb }} Some random [text with](https://fake.io) and more text ..."
            .to_string()
    }

    #[rstest]
    fn test_find_links_no_author_links() -> Result<()> {
        let s = "Some random text without link...";
        assert!(find_aipr_links(s).collect::<Vec<_>>() == vec![]);
        assert!(find_md_links(s).collect::<Vec<_>>() == vec![]);
        Ok(())
    }

    #[rstest]
    fn test_find_links_empty_link() -> Result<()> {
        let s = "Some random text with {{#colab  }} and {{}} {{#}}...";
        println!("{:?}", find_aipr_links(s).collect::<Vec<_>>());
        assert!(find_aipr_links(s).collect::<Vec<_>>() == vec![]);
        Ok(())
    }

    #[rstest]
    fn test_find_links_unknown_link_type() -> Result<()> {
        let s = "Some random \\[text with\\](test) {{#my_author ar.rs}} and {{#auth}} {{baz}} {{#bar}}...";
        assert!(find_aipr_links(s).collect::<Vec<_>>() == vec![]);
        assert!(find_md_links(s).collect::<Vec<_>>() == vec![]);
        Ok(())
    }

    #[rstest]
    fn test_find_links_simple_author_links(simple_book_content: String) -> Result<()> {
        let res = find_aipr_links(&simple_book_content[..]).collect::<Vec<_>>();
        println!("\nOUTPUT: {res:?}\n");

        assert_eq!(
            res,
            vec![
                AIPRLink {
                    start_index: 0,
                    end_index: 18,
                    link_type: AIPRLinkType::Header(AIPRHeaderSettings::default()),
                    link_text: "{{ #aipr_header }}",
                },
                AIPRLink {
                    start_index: 19,
                    end_index: 58,
                    link_type: AIPRLinkType::Header(AIPRHeaderSettings::from_param_str(
                        "colab=nlp/lora.ipynb"
                    )),
                    link_text: "{{ #aipr_header colab=nlp/lora.ipynb }}",
                },
            ]
        );
        Ok(())
    }

    #[rstest]
    #[case(
        "submit_issue=false,colab=nlp/lora.ipynb,reading_time=false",
        AIPRHeaderSettings {
            colab: Some("nlp/lora.ipynb".to_string()),
            submit_issue: false,
            reading_time: false
        }
    )]
    #[case(
        "colab=nlp/lora.ipynb",
        AIPRHeaderSettings {
            colab: Some("nlp/lora.ipynb".to_string()),
            ..Default::default()
        }
    )]
    #[case(
        "reading_time=falsee",
        AIPRHeaderSettings {
            ..Default::default()
        }
    )]
    fn test_aipr_header_settings(
        #[case] param_str: &str,
        #[case] expected_setting: AIPRHeaderSettings,
    ) -> Result<()> {
        let setting = AIPRHeaderSettings::from_param_str(param_str);
        assert_eq!(setting, expected_setting);

        Ok(())
    }

    #[rstest]
    fn test_link_render() -> Result<()> {
        let link = AIPRLink {
            start_index: 19,
            end_index: 58,
            link_type: AIPRLinkType::Header(AIPRHeaderSettings::from_param_str(
                "colab=nlp/lora.ipynb",
            )),
            link_text: "{{ #aipr_header colab=nlp/lora.ipynb }}",
        };
        let num_words = 201;

        let html_string = link.render(num_words)?;
        let expected = "<div style=\"display: flex; justify-content: \
        space-between; align-items: center; margin-bottom: 2em;\">\n  <div>\n    \
        <a target=\"_blank\" href=\"https://github.com/VectorInstitute/\
        ai-pocket-reference/issues/new?template=edit-request.yml\">\n      \
        <img src=\"https://img.shields.io/badge/Suggest_an_Edit-black?logo=\
        github&style=flat\" alt=\"Suggest an Edit\"/>\n    </a>\n    \
        <a target=\"_blank\" href=\"https://colab.research.google.com/github/\
        VectorInstitute/ai-pocket-reference-code/blob/main/notebooks/nlp/lora.ipynb\
        \">\n      <img src=\"https://colab.research.google.com/assets/colab-badge.svg\
        \" alt=\"Open In Colab\"/>\n    </a>\n    <p style=\"margin: 0;\">\
        <small>Reading time: 1 min</small></p>\n  </div>\n</div>\n";

        println!("{:#?}", html_string);

        assert_eq!(html_string, expected);

        Ok(())
    }

    #[rstest]
    fn test_link_render_no_colab() -> Result<()> {
        let link = AIPRLink {
            start_index: 19,
            end_index: 58,
            link_type: AIPRLinkType::Header(AIPRHeaderSettings::default()),
            link_text: "{{ #aipr_header }}",
        };
        let num_words = 301;

        let html_string = link.render(num_words)?;
        let expected = "<div style=\"display: flex; justify-content: \
        space-between; align-items: center; margin-bottom: 2em;\">\n  <div>\n    \
        <a target=\"_blank\" href=\"https://github.com/VectorInstitute/\
        ai-pocket-reference/issues/new?template=edit-request.yml\">\n      \
        <img src=\"https://img.shields.io/badge/Suggest_an_Edit-black?logo=\
        github&style=flat\" alt=\"Suggest an Edit\"/>\n    </a>\n    \
        <p style=\"margin: 0;\"><small>Reading time: 2 min</small></p>\n  \
        </div>\n</div>\n";

        assert_eq!(html_string, expected);

        Ok(())
    }

    #[rstest]
    fn test_link_render_no_colab_no_reading_time() -> Result<()> {
        let link = AIPRLink {
            start_index: 19,
            end_index: 58,
            link_type: AIPRLinkType::Header(AIPRHeaderSettings::from_param_str(
                "reading_time=false",
            )),
            link_text: "{{ #aipr_header reading_time=false }}",
        };
        let num_words = 200;

        let html_string = link.render(num_words)?;
        let expected = "<div style=\"display: flex; justify-content: \
        space-between; align-items: center; margin-bottom: 2em;\">\n  <div>\n    \
        <a target=\"_blank\" href=\"https://github.com/VectorInstitute/\
        ai-pocket-reference/issues/new?template=edit-request.yml\">\n      \
        <img src=\"https://img.shields.io/badge/Suggest_an_Edit-black?logo=\
        github&style=flat\" alt=\"Suggest an Edit\"/>\n    </a>\n  \
        </div>\n</div>\n";

        assert_eq!(html_string, expected);

        Ok(())
    }

    #[rstest]
    fn test_finds_md_link(simple_book_content: String) -> Result<()> {
        let res = find_md_links(&simple_book_content[..]).collect::<Vec<_>>();
        println!("\nOUTPUT: {res:?}\n");

        assert_eq!(
            res,
            vec![MDLink {
                start_index: 71,
                end_index: 99,
                text: "text with",
                url: "https://fake.io"
            }]
        );

        Ok(())
    }

    #[rstest]
    fn test_md_link_render() -> Result<()> {
        let link = MDLink {
            start_index: 19,
            end_index: 58,
            text: "some text",
            url: "https://fake.io",
        };

        let html_string = link.render()?;
        let expected = "<a href=\"https://fake.io\" target=\"_blank\" \
        rel=\"noopener noreferrer\">some text</a>\n";

        assert_eq!(html_string, expected);

        Ok(())
    }
}
