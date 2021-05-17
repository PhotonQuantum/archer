# Archer - a repository builder for ArchLinux

[![Test](https://github.com/PhotonQuantum/archer/actions/workflows/test.yml/badge.svg)](https://github.com/PhotonQuantum/archer/actions/workflows/test.yml)

> This project is at a very early stage.

## Current Progress

### Naive Dependency Resolving
![deps](demo.jpg)

## Todos
- [ ] dependency resolving
  + [ ] dfs search
    * [x] basic impl
    * [ ] nice error reporting
  + [x] graph output
    * [x] use petgraph impl
    * [x] migrate to custom impl
    * [x] custom dot output
  + [x] skip policy (ignore packages existing in certain repo)
  + [x] handle cyclic deps
  + [x] toposort & SCC
    * [x] basic impl
    * [x] complete impl
  + [x] support for makedepends
  + [x] batch query
  + [x] parallel query for aur
  + [ ] custom pkgbuild support
    * [ ] basic impl
    * [ ] support .SRCINFO
  + [ ] plan builder
    * [x] basic impl
    * [ ] complete impl
  + [ ] unittest
    * [ ] package
    * [x] repository
    * [x] basic resolve
    * [ ] cyclic deps
    * [ ] plan builder
    * [ ] parser
- [ ] build environment setup
  + [ ] bare metal
  + [ ] bubblewrap
  + [ ] official container buildtools
- [ ] build workflow
  + [ ] split package
  + [ ] error handling
- [ ] storage support
  + [ ] file
  + [ ] aliyun oss
  + [ ] s3
- [ ] update checker
  + [ ] support for vcs packages
- [ ] metadata & build report (json, plain)
  + [ ] basic functionality
  + [ ] frontend (optional)