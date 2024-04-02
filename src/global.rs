// Copyright 2023 RisingWave Labs
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

use std::sync::OnceLock;

use crate::{Config, Registry};

static GLOBAL_REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Initialize the global registry with the given configuration.
/// Panics if the global registry has already been initialized.
///
/// This is **optional** and only needed if you want to use the global registry.
/// You can always create a new registry with [`Registry::new`] and pass it around to achieve
/// better encapsulation.
pub fn init_global_registry(config: Config) {
    if let Err(_r) = GLOBAL_REGISTRY.set(Registry::new(config)) {
        panic!("global registry already initialized")
    }
}

pub(crate) fn global_registry() -> Option<Registry> {
    GLOBAL_REGISTRY.get().cloned()
}
