use mdbook::MDBook;

#[test]
fn aipr_header_works() {
    // Tests that the ai-pocket-reference::aipr_headers example works as expected.

    // Workaround for https://github.com/rust-lang/mdBook/issues/1424
    std::env::set_current_dir("test_book").unwrap();
    let book = MDBook::load(".").unwrap();
    book.build().unwrap();
    let ch1 = std::fs::read_to_string("book/chapter_1/index.html").unwrap();
    let ch1_1 = std::fs::read_to_string("book/chapter_1/sub_chapter_1.html").unwrap();
    let ch1_2 = std::fs::read_to_string("book/chapter_1/sub_chapter_2.html").unwrap();

    // chapter 1
    assert!(
        ch1.contains("https://img.shields.io/badge/Suggest_an_Edit-black?logo=github&style=flat")
    );
    assert!(ch1.contains("<small>Reading time: "));
    assert!(!ch1.contains("blob/main/notebooks/"));
    // chapter 1.1
    assert!(
        ch1_1.contains("https://img.shields.io/badge/Suggest_an_Edit-black?logo=github&style=flat")
    );
    assert!(!ch1_1.contains("blob/main/notebooks/nlp/attention.ipynb"));
    // chapter 1.2
    assert!(
        ch1_2.contains("https://img.shields.io/badge/Suggest_an_Edit-black?logo=github&style=flat")
    );
    assert!(!ch1_2.contains("blob/main/notebooks/"));
    assert!(!ch1_2.contains("<small>Reading time: "));
}
