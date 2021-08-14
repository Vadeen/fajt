//! This file generates test cases from "snapshot" files.
//! The snapshot files contains javascript and the AST we expect that javascript to be parsed to.
//!
//! In our case, the snapshot files are .md files and contain both javascript and json in the same
//! file. The .md file format was chosen because many editors and viewers have support for syntax
//! highlighting of both javascript and json in the same file.
//!
//! Each test file generates a test named as the file name with some characters replaced.
//! For example:
//!     folder/new-empty-args.md
//! will generate the test case:
//!     folder__new_empty_args
//! this test is generated by a procedural macro and can be run as any other test.
//!
//! Generation of AST
//! It's very cumbersome to write the json AST by hand, to make testing easier you can simply put
//! the js part in the .md file, then run the test. The test will fail the first time but append
//! the json AST to the .md file. Verify that the ast is correct and rerun.
//!
//! The generation is also for future compatibility. When the ast changes, we can regenerate all
//! ASTs in the test cases and just verify the diff instead of manually refactoring hundreds of
//! test assertions.
extern crate fajt_macros;

use fajt_lexer::Lexer;
use fajt_parser::error::{ErrorKind, Result};
use fajt_parser::parser::Parse;
use fajt_parser::Parser;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// TODO possibility to regenerate all asts.

macro_rules! generate_test_cases {
    ("md", $file_path:literal, $ident:ident) => {
        #[test]
        fn $ident() {
            snapshot_runner($file_path)
        }
    };
    ("md_ignore", $file_path:literal, $ident:ident) => {
        #[ignore]
        #[test]
        fn $ident() {
            snapshot_runner($file_path)
        }
    };
    ($extension:literal, $file_path:literal, $ident:ident) => {};
}

macro_rules! generate_test_module {
    (
        mod_name: $mod_name:ident,
        ast_type: $ast_type:ident,
        folders: [$( $folder:literal ),*],
    ) => {
        /// Everything inside snapshots/expr is parsed as expressions.
        mod $mod_name {
            use super::{md, parse_input, evaluate_result};
            use fajt_macros::for_each_file;
            use fajt_parser::ast::$ast_type;

            fn snapshot_runner(test_file: &str) {
                println!("Running: {}", test_file);

                let markdown = md::Markdown::from_file(test_file.as_ref());
                let result = parse_input::<$ast_type>(&markdown.js_block);
                evaluate_result(result, &markdown);
            }

            $(
                for_each_file!($folder, generate_test_cases);
            )*
        }
    }
}

generate_test_module!(
    mod_name: expr,
    ast_type: Expr,
    folders: ["parser/tests/snapshots/expr"],
);

generate_test_module!(
    mod_name: stmt,
    ast_type: Stmt,
    folders: ["parser/tests/snapshots/stmt"],
);

generate_test_module!(
    mod_name: decl,
    ast_type: Stmt,
    folders: ["parser/tests/snapshots/decl"],
);

generate_test_module!(
    mod_name: semicolon,
    ast_type: Program,
    folders: ["parser/tests/snapshots/semicolon"],
);

fn evaluate_result<'a, 'b: 'a, T>(result: Result<T>, markdown: &'b md::Markdown)
where
    T: Deserialize<'a> + Serialize + PartialEq + Debug,
{
    if let Some(expected_data) = &markdown.json_block {
        if let Ok(result) = result {
            let expected_expr: T = serde_json::from_str(&expected_data).unwrap();
            assert_eq!(result, expected_expr)
        } else {
            let expected_error: ErrorKind = serde_json::from_str(&expected_data).unwrap();
            assert_eq!(result.unwrap_err().kind(), &expected_error)
        }
    } else {
        if let Ok(result) = result {
            let json = serde_json::to_string_pretty(&result).unwrap();
            markdown.append_json_block(&json);
            panic!("No ast found in this test. Json generated, verify and rerun.");
        } else {
            let error = serde_json::to_string_pretty(&result.unwrap_err().kind()).unwrap();
            markdown.append_json_block(&error);
            panic!("No ast found in this test. Json error generated, verify and rerun.");
        }
    }
}

fn regenerate_asts<'a, 'b: 'a, T>(result: Result<T>, markdown: &'b md::Markdown)
where
    T: Deserialize<'a> + Serialize + PartialEq + Debug,
{
    if let Ok(result) = result {
        let json = serde_json::to_string_pretty(&result).unwrap();
        markdown.replace_json_block(&json)
    } else {
        let json = serde_json::to_string_pretty(&result.unwrap_err().kind()).unwrap();
        markdown.replace_json_block(&json)
    }
}

fn parse_input<T>(input: &str) -> Result<T>
where
    T: Parse,
{
    let lexer = Lexer::new(input).unwrap();
    let mut reader = fajt_common::io::PeekReader::new(lexer).unwrap();
    Parser::parse::<T>(&mut reader)
}

// TODO clean up this module
mod md {
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::{Path, PathBuf};

    pub struct Markdown {
        path: PathBuf,
        pub js_block: String,
        pub json_block: Option<String>,
    }

    impl Markdown {
        pub fn from_file(path: &Path) -> Self {
            let data = read_string(path);

            let js_block = get_code_block(&data, "js")
                .expect("JS input required.")
                .to_owned();
            let json_block = get_code_block(&data, "json").map(&str::to_owned);
            Markdown {
                path: PathBuf::from(path),
                js_block,
                json_block,
            }
        }

        pub fn append_json_block(&self, data: &str) {
            let block = generate_code_block(&data, "json");

            let mut file = OpenOptions::new().write(true).open(&self.path).unwrap();

            file.seek(SeekFrom::End(0)).unwrap();
            file.write_all("\n\n".as_bytes()).unwrap();
            file.write_all(block.as_bytes()).unwrap();
        }

        pub fn replace_json_block(&self, contents: &str) {
            let data = read_string(&self.path);

            if let Some((start, end)) = get_code_block_pos(&data, "json") {
                let mut new_data = String::new();
                new_data.push_str(&data[..start]);
                new_data.push_str(contents);
                new_data.push_str(&data[end - 1..]);

                let mut file = OpenOptions::new().write(true).open(&self.path).unwrap();
                file.write_all(new_data.as_bytes()).unwrap();
            }
        }
    }

    const BLOCK_DELIMITER: &str = "```";

    fn get_code_block<'a>(source: &'a str, annotation: &str) -> Option<&'a str> {
        let pos = get_code_block_pos(source, annotation);
        pos.map(|(start, end)| &source[start..end])
    }

    fn get_code_block_pos(source: &str, annotation: &str) -> Option<(usize, usize)> {
        let block_start = format!("{}{}\n", BLOCK_DELIMITER, annotation);
        if let Some(start) = source.find(&block_start) {
            // Block start without preceding new line is only valid if block starts at first line.
            if start != 0 && &source[start - 1..start] != "\n" {
                return None;
            }

            // The data starts after the start pattern.
            let start = start + block_start.len();
            (&source[start..]).find(BLOCK_DELIMITER).map(|end| {
                // \n``` without a new line after is only valid if file ends
                (start, end + start)
            })
        } else {
            None
        }
    }

    fn generate_code_block(data: &str, annotation: &str) -> String {
        format!(
            "{}{}\n{}\n{}\n",
            BLOCK_DELIMITER, annotation, data, BLOCK_DELIMITER
        )
    }

    fn read_string(path: &Path) -> String {
        let mut file = File::open(path).expect("Failed to open file.");
        let mut data = String::new();
        file.read_to_string(&mut data)
            .expect("Failed to read file.");
        data
    }
}
