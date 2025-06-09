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
