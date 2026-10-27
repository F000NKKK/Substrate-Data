//! The READMEs' examples, compiled as doc tests so they cannot rot.

#[doc = include_str!("../README.md")]
struct CrateReadme;

#[doc = include_str!("../../../README.md")]
struct RepositoryReadme;
