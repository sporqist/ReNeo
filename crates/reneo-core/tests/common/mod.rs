//! Helpers shared by the integration tests

#![allow(dead_code)]

use std::path::PathBuf;

use reneo_core::keysym::Keysyms;

pub fn repo_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(name)
}

pub fn keysyms() -> Keysyms {
    Keysyms::parse(&std::fs::read_to_string(repo_file("keysymdef.h")).unwrap())
}
