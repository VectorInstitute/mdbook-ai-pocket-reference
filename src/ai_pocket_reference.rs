use std::collections::HashMap;

use mdbook::book::Book;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use once_cell::sync::Lazy;
use regex::{CaptureMatches, Captures, Regex};

#[derive(Default)]
pub struct AIPRPreprocessor;

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

    fn run(&self, _ctx: &PreprocessorContext, book: Book) -> anyhow::Result<Book> {
        Ok(book)
    }
}

#[allow(dead_code)]
fn replace_all(s: &str) -> String {
    // When replacing one thing in a string by something with a different length,
    // the indices after that will not correspond,
    // we therefore have to store the difference to correct this
    let mut previous_end_index = 0;
    let mut replaced = String::new();

    for link in find_aipr_links(s) {
        replaced.push_str(&s[previous_end_index..link.start_index]);
        previous_end_index = link.end_index;
    }

    replaced.push_str(&s[previous_end_index..]);
    replaced
}

#[allow(dead_code)]
#[derive(PartialEq, Debug, Clone)]
enum AIPRLinkType {
    Header(AIPRHeaderSettings),
}

#[allow(dead_code)]
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
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rstest::*;

    #[fixture]
    fn simple_book_content() -> String {
        "{{ #aipr_header }} {{ #aipr_header colab=nlp/lora.ipynb }} Some random text with and more text ..."
            .to_string()
    }

    #[rstest]
    fn test_find_links_no_author_links() -> Result<()> {
        let s = "Some random text without link...";
        assert!(find_aipr_links(s).collect::<Vec<_>>() == vec![]);
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
        let s = "Some random text with {{#my_author ar.rs}} and {{#auth}} {{baz}} {{#bar}}...";
        assert!(find_aipr_links(s).collect::<Vec<_>>() == vec![]);
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
        "submit_issue=false,colab=nlp/lora.ipynb,reading_time=true",
        AIPRHeaderSettings { colab: Some("nlp/lora.ipynb".to_string()), submit_issue: false, ..Default::default()})
    ]
    fn test_aipr_header_settings(
        #[case] param_str: &str,
        #[case] expected_setting: AIPRHeaderSettings,
    ) -> Result<()> {
        let setting = AIPRHeaderSettings::from_param_str(param_str);
        assert_eq!(setting, expected_setting);

        Ok(())
    }
}
