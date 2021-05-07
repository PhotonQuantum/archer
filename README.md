# Archer - a repository builder for ArchLinux

> This project is at a very early stage.

## Current Progress

### Naive Dependency Resolving
![deps](demo.jpg)

## Todos
- [ ] dependency resolving
  + [x] dfs search (find dependency closure)
  + [x] graph output (via petgraph)
  + [x] skip policy (ignore packages existing in certain repo)
  + [x] topo sort
  + [x] support for makedepends
  + [x] batch query
  + [x] parallel query for aur
  + [ ] custom pkgbuild support
  + [ ] plan builder
- [ ] build environment setup
  + [ ] bare metal
  + [ ] bubblewrap
  + [ ] official container buildtools
- [ ] build workflow
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