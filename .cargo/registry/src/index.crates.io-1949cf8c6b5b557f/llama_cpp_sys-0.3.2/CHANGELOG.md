# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.2.2 (2023-11-08)

### Bug Fixes

 - <csr-id-674f395961c27f1f1d53d487721e24f04fc81d71/> do not rerun build on changed header files
   this restores functionality lost in the latest upgrade to `bindgen`, which enabled this functionality

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Do not rerun build on changed header files ([`674f395`](https://github.com/binedge/llama_cpp-rs/commit/674f395961c27f1f1d53d487721e24f04fc81d71))
    - Release llama_cpp_sys v0.2.1, llama_cpp v0.1.1 ([`a9e5813`](https://github.com/binedge/llama_cpp-rs/commit/a9e58133cb1c1d4d45f99a7746e0af7da1a099e1))
</details>

## v0.2.1 (2023-11-08)

<csr-id-ccb794d346de87e48199f9f0f3564f3c7a2cd607/>

### Chore

 - <csr-id-ccb794d346de87e48199f9f0f3564f3c7a2cd607/> Update to `bindgen` 0.69.1

### Chore

 - <csr-id-6d3183d1e6c2df98b8b3a2db405d6af163ca582a/> Update to `bindgen` 0.69.1

### Bug Fixes

 - <csr-id-4eb0bc9800877e460fe0d1d25398f35976b4d730/> `start_completing` should not be invoked on a per-iteration basis
   There's still some UB that can be triggered due to llama.cpp's threading model, which needs patching up.
 - <csr-id-27706de1a471b317e4b7b4fdd4c5bbabfbd95ed6/> `start_completing` should not be invoked on a per-iteration basis
   There's still some UB that can be triggered due to llama.cpp's threading model, which needs patching up.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 13 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release llama_cpp_sys v0.2.1, llama_cpp v0.1.1 ([`ef4e3f7`](https://github.com/binedge/llama_cpp-rs/commit/ef4e3f7a3c868a892f26acfae2a5211de4900d1c))
    - Update to `bindgen` 0.69.1 ([`6d3183d`](https://github.com/binedge/llama_cpp-rs/commit/6d3183d1e6c2df98b8b3a2db405d6af163ca582a))
    - `start_completing` should not be invoked on a per-iteration basis ([`27706de`](https://github.com/binedge/llama_cpp-rs/commit/27706de1a471b317e4b7b4fdd4c5bbabfbd95ed6))
    - Update to `bindgen` 0.69.1 ([`ccb794d`](https://github.com/binedge/llama_cpp-rs/commit/ccb794d346de87e48199f9f0f3564f3c7a2cd607))
    - `start_completing` should not be invoked on a per-iteration basis ([`4eb0bc9`](https://github.com/binedge/llama_cpp-rs/commit/4eb0bc9800877e460fe0d1d25398f35976b4d730))
</details>

## v0.2.0 (2023-10-25)

<csr-id-116fe8c82fe2c43bf9041f6dbfe2ed15d00e18e9/>
<csr-id-96548c840d3101091c879648074fa0ed1cee3011/>
<csr-id-2d14d8df7e3850525d0594d387f65b7a4fc26646/>
<csr-id-a5fb19499ecbb1060ca8211111f186efc6e9b114/>

### Chore

 - <csr-id-116fe8c82fe2c43bf9041f6dbfe2ed15d00e18e9/> Release
 - <csr-id-96548c840d3101091c879648074fa0ed1cee3011/> latest fixes from upstream

### Bug Fixes

 - <csr-id-b9cde4a7a09837f7b01b124acb8325391e3b1b65/> set clang to use c++ stl
 - <csr-id-2cb06aea62b892a032f515b78d720acb915f4a22/> use SPDX license identifiers

### Other

 - <csr-id-2d14d8df7e3850525d0594d387f65b7a4fc26646/> use `link-cplusplus`, enable build+test on all branches
   * ci: disable static linking of llama.o
   
   * ci: build+test on all branches/prs
   
   * ci: use `link-cplusplus`
 - <csr-id-a5fb19499ecbb1060ca8211111f186efc6e9b114/> configure for `cargo-release`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 5 calendar days.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1](https://github.com/binedge/llama_cpp-rs/issues/1), [#2](https://github.com/binedge/llama_cpp-rs/issues/2), [#3](https://github.com/binedge/llama_cpp-rs/issues/3)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1](https://github.com/binedge/llama_cpp-rs/issues/1)**
    - Use `link-cplusplus`, enable build+test on all branches ([`2d14d8d`](https://github.com/binedge/llama_cpp-rs/commit/2d14d8df7e3850525d0594d387f65b7a4fc26646))
 * **[#2](https://github.com/binedge/llama_cpp-rs/issues/2)**
    - Prepare for publishing to crates.io ([`f35e282`](https://github.com/binedge/llama_cpp-rs/commit/f35e28252ec7817a8999b83bdac33dffebf4b663))
 * **[#3](https://github.com/binedge/llama_cpp-rs/issues/3)**
    - Release ([`116fe8c`](https://github.com/binedge/llama_cpp-rs/commit/116fe8c82fe2c43bf9041f6dbfe2ed15d00e18e9))
 * **Uncategorized**
    - Release llama_cpp_sys v0.2.0 ([`fa3af83`](https://github.com/binedge/llama_cpp-rs/commit/fa3af83e51f552ad60e3f4e06cb3582b0cb4be2f))
    - Use SPDX license identifiers ([`2cb06ae`](https://github.com/binedge/llama_cpp-rs/commit/2cb06aea62b892a032f515b78d720acb915f4a22))
    - Release llama_cpp_sys v0.2.0 ([`85f21a1`](https://github.com/binedge/llama_cpp-rs/commit/85f21a1eca80faa9bd3f2f160d58b21a437814aa))
    - Add CHANGELOG.md ([`0e836f5`](https://github.com/binedge/llama_cpp-rs/commit/0e836f5b60b0e2f110972ef384f23c350150f55b))
    - Set clang to use c++ stl ([`b9cde4a`](https://github.com/binedge/llama_cpp-rs/commit/b9cde4a7a09837f7b01b124acb8325391e3b1b65))
    - Latest fixes from upstream ([`96548c8`](https://github.com/binedge/llama_cpp-rs/commit/96548c840d3101091c879648074fa0ed1cee3011))
    - Configure for `cargo-release` ([`a5fb194`](https://github.com/binedge/llama_cpp-rs/commit/a5fb19499ecbb1060ca8211111f186efc6e9b114))
    - Initial commit ([`6f672ff`](https://github.com/binedge/llama_cpp-rs/commit/6f672ffddc49ce23cd3eb4996128fe8614c560b4))
</details>

