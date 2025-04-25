// Copyright (c) 2020, Kevin Schibli
// All rights reserved.

// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:

// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.

// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.

// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.

// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use pkg_config::{Config, Error};
use std::env::{set_var, var_os};
use std::fs::{canonicalize, copy, create_dir_all};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let min_version = "2.30.0";
    let library_name = "libisal";

    let lib_isal = match Config::new()
        .atleast_version(min_version)
        .probe(library_name)
    {
        Ok(lib_isal) => lib_isal,
        Err(error) => match error {
            Error::Failure { .. } | Error::ProbeFailure { .. } => {
                // Submodule directory containing upstream source files (readonly)
                let submodule_dir =
                    canonicalize("./isa-l").expect("isa-l submodule directory not found");

                // Copy source files to writable directory in `OUT_DIR`
                let out_src_dir = PathBuf::from(var_os("OUT_DIR").unwrap()).join("src");
                create_dir_all(&out_src_dir).expect("Failed to create $OUT_DIR/src");
                cp_r(submodule_dir, out_src_dir.clone());

                // Run `./autogen.sh`
                Command::new("sh")
                    .current_dir(out_src_dir.clone())
                    .arg("autogen.sh")
                    .status()
                    .unwrap();

                // Build using autotools
                let install_root_dir = autotools::build(out_src_dir);

                // Set install directory as env var to be read by package config
                set_var("PKG_CONFIG_PATH", install_root_dir.join("lib/pkgconfig"));

                // Probe static library
                Config::new()
                    .atleast_version(min_version)
                    .probe(library_name)
                    .expect("Static library built from source not found")
            }
            _ => {
                panic!("{}", error);
            }
        },
    };

    for include in &lib_isal.include_paths {
        println!("cargo:root={}", include.display());
    }
}

fn cp_r(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    for e in from.as_ref().read_dir().unwrap() {
        let e = e.unwrap();
        let from = e.path();
        let to = to.as_ref().join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            create_dir_all(&to).unwrap();
            cp_r(&from, &to);
        } else {
            println!("{} => {}", from.display(), to.display());
            copy(&from, &to).unwrap();
        }
    }
}
