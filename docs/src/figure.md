# The `mdbook-figure` preprocessor

This preprocessor allows you to define figures with a label, an optional type, a description, and contents.

## Quick Start

To get started, install the preprocessor:

```sh
cargo install mdbook-figure
```

and add it to your `book.toml`:

```toml
[book]

[preprocessor.figure]
# If you're also using `mdbook-crossref`, you must use the correct ordering:
before = [ "crossref" ]
```

## Defining figures

Defining figures is done as follows:

````
```figure a-label Optional Type
The first line is the description of the figure
<center>
The rest describes its contents, which are rendered as

**markdown** _in_ `the` final document.
</center>
```
````

which renders as

```figure a-label Optional Type
The first line is the description of the figure
<center>
The rest describes its contents, which are rendered as

**markdown** _in_ `the` final document.
</center>
```

The figures are numbered by type and order in the book.

These figures can be referred to by their label using the `mdbook-crossref` preprocessor. In this case, we can refer to [`ref:a-label`](ref:a-label), or <ref:a-table>.

## Autodetection

If the type of the figure is not specified, it defaults to "Figure". However, the type is automatically inferred for some content, so long as that
content immediately follows the description.

Currently, only "Table" is supported:

````
```figure a-table
a very fancy table
| Column 1 | Column 2 |
| :------- | :------- |
| Value1   | Value2   |
```
````

which renders as

```figure a-table
a very fancy table
| Column 1 | Column 2 |
| :------- | :------- |
| Value1   | Value2   |
```

and is referred to as <ref:a-table>

```figure a-table
A duplicated table figure
```