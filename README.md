# XivAr

[![crates.io](https://img.shields.io/crates/v/xivar.svg)](https://crates.io/crates/xivar)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Mountlex/xivar/Rust)
![actively developed](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![dependency status](https://deps.rs/crate/xivar/0.3.2/status.svg)](https://deps.rs/crate/xivar/0.3.1)
![License: MIT/Apache-2.0](https://img.shields.io/crates/l/xivar.svg)

Manage your local scientific library!

**This tool is in a very early development stage.**

## Installation

First, install [fzf](https://github.com/junegunn/fzf). **_(required)_**

Then install `xivar` via

```bash
cargo install xivar
```

## Usage

`xivar` lets you search publications online at [DBLP](https://dblp.org/) and [arXiv](https://arxiv.org/), and open or download them if available. `xivar` saves the locations of downloaded publications and shows them in your next search.

### Search

Search a publication online and local.

```bash
xivar search keyword1 keyword2 ...
```

Search only locally saved files.

```bash
xivar local keyword1 keyword2 ...
```

### Clean

Clean your database, i.e. remove entries of publications which cannot be found at their saved location.

```bash
xivar clean
```

### Add

Add a local pdf to the database.

```bash
xivar add paper.pdf
```

If `xivar` finds any metadata in the PDF-file, you can confirm to search the title online. If you can find the matching paper online, all necessary information will be fetched and written to your library. Otherwise, you have to manually enter them.

## Configuration

In Linux-based systems, the database is located at `~/.local/share/xivar`.
**Since this is still very much work in progress, the database may be corrupt after updating to a new version!**

You can configure the default download location via a configuration file located at `~/.config/xivar/xivar.toml` with the following content

```toml
document_dir = "absolute/path/to/directory"
```

## Roadmap

- Update library
- Export bib-files
- Specify query more precisely (title, author, AND, OR etc.)
- ...
