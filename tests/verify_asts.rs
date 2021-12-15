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

mod markdown;

use fajt_ast::SourceType;
use fajt_parser::error::{ErrorKind, Result};
use fajt_parser::{parse, Parse};
use markdown::TestFile;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

// TODO possibility to regenerate all asts.

// This runs for each .md file in the ./cases folder.
fn run_test_file<T>(path: &str, source_type: SourceType)
where
    T: Parse + Serialize + DeserializeOwned + PartialEq + Debug,
{
    println!("Running: {}", path);

    let test_file = TestFile::from(&path);
    let result = parse::<T>(&test_file.source, source_type);

    if test_file.ast.is_none() {
        // If the test file contain no output, we generate that from result of running the code.
        // I.e. you can add a test file with just code to generate the result.
        generate_expected_output(result, test_file);
        panic!("No ast found in this test. Output generated, verify and rerun.");
    }

    let ast = test_file.ast.unwrap();
    assert_result(result, ast);
}

fn assert_result<T>(result: Result<T>, ast_json: String)
    where
        T: Parse + Serialize + DeserializeOwned + PartialEq + Debug,
{
    if let Ok(result) = result {
        let expected_expr: T = serde_json::from_str(&ast_json).unwrap();
        assert_eq!(result, expected_expr)
    } else {
        let error = result.unwrap_err();
        println!("Error: {:?}", error);

        let expected_error: ErrorKind = serde_json::from_str(&ast_json).unwrap();
        assert_eq!(error.kind(), &expected_error)
    }
}

fn generate_expected_output<T>(result: Result<T>, test_file: TestFile)
where
    T: Parse + Serialize + Debug
{
    if let Ok(result) = result {
        let json = serde_json::to_string_pretty(&result).unwrap();
        test_file.append_json_block(&json);
    } else {
        let error = serde_json::to_string_pretty(&result.unwrap_err().kind()).unwrap();
        test_file.append_json_block(&error);
    }
}

#[allow(unused)]
fn regenerate_asts<T>(result: Result<T>, test_file: TestFile)
    where
        T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    if let Ok(result) = result {
        let json = serde_json::to_string_pretty(&result).unwrap();
        test_file.replace_json_block(&json)
    } else {
        let json = serde_json::to_string_pretty(&result.unwrap_err().kind()).unwrap();
        test_file.replace_json_block(&json)
    }
}

macro_rules! generate_test_cases {
    ("md", $file_path:literal, $ident:ident) => {
        #[test]
        fn $ident() {
            run_test($file_path)
        }
    };
    ("md_ignore", $file_path:literal, $ident:ident) => {
        #[ignore]
        #[test]
        fn $ident() {
            run_test($file_path)
        }
    };
    ($extension:literal, $file_path:literal, $ident:ident) => {};
}

macro_rules! generate_test_module {
    (
        mod_name: $mod_name:ident,
        ast_type: $ast_type:ident,
        source_type: $source_type:ident,
        folders: [$( $folder:literal ),*],
    ) => {
        mod $mod_name {
            use fajt_macros::for_each_file;
            use fajt_ast::$ast_type;
            use fajt_ast::SourceType::$source_type;

            fn run_test(test_file: &str) {
                $crate::run_test_file::<$ast_type>(&test_file, $source_type);
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
    source_type: Script,
    folders: ["tests/cases/expr"],
);

generate_test_module!(
    mod_name: stmt,
    ast_type: Stmt,
    source_type: Script,
    folders: ["tests/cases/stmt"],
);

generate_test_module!(
    mod_name: decl,
    ast_type: Stmt,
    source_type: Script,
    folders: ["tests/cases/decl"],
);

generate_test_module!(
    mod_name: semicolon,
    ast_type: Program,
    source_type: Unknown,
    folders: ["tests/cases/semicolon"],
);

generate_test_module!(
    mod_name: strict_mode,
    ast_type: Program,
    source_type: Script,
    folders: ["tests/cases/strict-mode"],
);

generate_test_module!(
    mod_name: source_module,
    ast_type: Program,
    source_type: Module,
    folders: ["tests/cases/source-module"],
);

generate_test_module!(
    mod_name: source_script,
    ast_type: Program,
    source_type: Script,
    folders: ["tests/cases/source-script"],
);

#[test]
fn dummy() {
    // This is just so IDE recognize this is a runnable file.
}
