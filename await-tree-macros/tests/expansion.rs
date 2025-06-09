// Copyright 2025 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test to verify the macro expansion works correctly.

use await_tree_macros::instrument;

// Test that the macro generates the expected code structure
#[instrument("test_expansion({})", value)]
async fn test_expansion(value: i32) -> i32 {
    value * 2
}

#[tokio::test]
async fn test_macro_expansion() {
    // This test verifies that the macro expansion compiles and runs correctly
    let result = test_expansion(21).await;
    assert_eq!(result, 42);
}

// Test with no arguments
#[instrument]
async fn no_args_function() -> String {
    "success".to_string()
}

#[tokio::test]
async fn test_no_args_expansion() {
    let result = no_args_function().await;
    assert_eq!(result, "success");
}

// Test with long_running keyword
#[instrument(long_running, "long_running_task({})", id)]
async fn long_running_task(id: u32) -> u32 {
    id * 10
}

// Test with verbose keyword
#[instrument(verbose, "verbose_task")]
async fn verbose_task() -> String {
    "verbose".to_string()
}

// Test with both keywords
#[instrument(long_running, verbose, "complex_task({}, {})", name, value)]
async fn complex_task(name: &str, value: i32) -> String {
    format!("{}: {}", name, value)
}

// Test with keywords but no format args
#[instrument(long_running, verbose)]
async fn keywords_only_task() -> i32 {
    42
}

#[tokio::test]
async fn test_keywords() {
    let result = long_running_task(5).await;
    assert_eq!(result, 50);

    let result = verbose_task().await;
    assert_eq!(result, "verbose");

    let result = complex_task("test", 123).await;
    assert_eq!(result, "test: 123");

    let result = keywords_only_task().await;
    assert_eq!(result, 42);
}

// Note: The macro now accepts any identifiers as method names.
// If the methods don't exist on Span, it will fail at compile time, which is the desired behavior.
// For example, this would fail to compile:
// #[instrument(custom_method, another_method, "arbitrary_methods")]
// async fn arbitrary_methods_task() -> String { "this compiles".to_string() }
