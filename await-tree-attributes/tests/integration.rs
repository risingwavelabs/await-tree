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

use await_tree_attributes::instrument;

// Test basic usage with format string and arguments
#[instrument("test_function({})", arg1)]
async fn test_function(arg1: i32, arg2: String) -> i32 {
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    arg1 + arg2.len() as i32
}

// Test with no arguments (should use function name)
#[instrument]
async fn simple_function() -> String {
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    "hello".to_string()
}

// Test with complex format string
#[instrument("complex_operation({}, {})", name, value)]
async fn complex_function(name: &str, value: u64) -> String {
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    format!("{}: {}", name, value)
}

#[tokio::test]
async fn test_instrument_attribute() {
    // These tests mainly verify that the attribute compiles correctly
    // and the functions can be called normally

    let result = test_function(42, "test".to_string()).await;
    assert_eq!(result, 46);

    let result = simple_function().await;
    assert_eq!(result, "hello");

    let result = complex_function("test", 123).await;
    assert_eq!(result, "test: 123");
}

// Test that the macro preserves function visibility and attributes
#[instrument("public_fn")]
pub async fn public_function() -> i32 {
    42
}

#[allow(dead_code)]
#[instrument("private_fn")]
async fn private_function() -> i32 {
    24
}

#[tokio::test]
async fn test_visibility_and_attributes() {
    let result = public_function().await;
    assert_eq!(result, 42);

    let result = private_function().await;
    assert_eq!(result, 24);
}
