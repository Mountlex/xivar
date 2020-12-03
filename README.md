# XivAr

Manage your local scientific library!

**This tool is in a very early development stage.**

## Installation

First, install [fzf](https://github.com/junegunn/fzf). Then install `xivar` via

```bash
cargo install xivar
```

## Usage

`xivar` lets you search publications online at [DBLP](https://dblp.org/), and download them from a preprint server (arXiv etc.) if available. `xivar` saves the locations of downloaded publications and shows them in your next search.

Search a publication online and local

```bash
xivar search keyword1 keyword2 ...
```

Search only local

```bash
xivar local keyword1 keyword2 ...
```

Clean your database, i.e. remove entries of publications which cannot be found at their saved location.

```bash
xivar clean
```

## Configuration

In Linux-based systems, the database is located at `~/.local/share/xivar`.
**Since this is still very much work in progress, the database may be corrupt after updating!**

TODO

## Roadmap

- Scan your computer for pdf files and import them
- Export bib-files
- ...
