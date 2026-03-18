# Quick Start { #quickstart }
[](label:quickstart "the quickstart chapter")

To get started, install the preprocessor:

```sh
cargo install mdbook-crossref
```

and add it to your `book.toml`:

```toml
[book]

[preprocessor.crossref] # This is all you need to do
```

and start creating `label`s and `ref`erences.

That's all there is to it! To find out how to actually use the preprocessor in more
detail, see <ref:creating_links>.

```figure text-figure
Description
|  A table  |
| :-------: |
| Waah waah |
```

```figure text-figure
Description
|  A table  |
| :-------: |
| Waah waah |
```

```figure text-figure
Description
|  A table  |
| :-------: |
| Waah waah |
```
<ref:text-figure>