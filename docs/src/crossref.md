# The `mdbook-xref` preprocessor { #creating_links }
[](label:creating_links "creating links")

This preprocessor allow you to create cross-referenceable labels by creating new link items
that use the `label` protocol. Additionally, it lets you set the supplement (the text substituted
at the place of reference) for these links, which is used unless the reference specifies its text
explicitly.

The created labels can then be referred to using links with the `ref` protocol.

## Quick Start

To get started, install the preprocessor:

```sh
cargo install mdbook-xref
```

and add it to your `book.toml`:

```toml
[book]

[preprocessor.xref] # This is all you need to do
```

and start creating `label`s and `ref`erences.

That's all there is to it! To find out how to actually use the preprocessor in more
detail, see <ref:creating_links>.


## Creating cross-referenceable labels

A piece of referenceable text (with support for inline markdown) can be created as follows:

```
[a piece of referenceable text](label:a_piece_of_text "an optional supplement")

[a piece of **referenceable** _text_ without supplement, but nice markdown](label:text_without_supplement)
```

which is rendered like this:

[a piece of referenceable text](label:a_piece_of_text "an optional supplement")

[a piece of **referenceable** _text_ without supplement, but nice markdown](label:text_without_supplement)

## Cross-referencing other elements

Non-text elements can also be cross-referenced as follows:

```
##### A referenceable heading { #heading }
[](label:heading "the heading's supplement")
```

Labels without any text are not rendered, but cross-references to `heading`
will create a link to the element with ID of the label, which in this case is `heading`
(using `mdBooks` built-in heading ID support).

It is rendered like this (note that the labelled data is not explicitly visible):

##### A referenceable heading { #heading }
[](label:heading "the heading's supplement")

In general, you should not create cross-referenceable items like this manually. Instead, you
should let other preprocessors generate them for you.

## Referring to cross-referefences { #creating }
[](label:creating "the section on creating references")

A cross reference can be referred to by any links with the `ref` protocol followed by a
label defined in some `ref` link.

### Normal inline links

Normal inline links work as expected

```
[A reference to the text.](label:text_without_supplement)
```

renders as

[A reference to the text.](ref:text_without_supplement)

### Reference links

Reference links are supported:

```
[A reference to the text][1]

[1]: ref:a_piece_of_text
```

renders as

[A reference to the text][1]

[1]: ref:a_piece_of_text

### Autolinks

Autolinks are also supported, but only work if the reference has a supplement:

```
<ref:a_piece_of_text>

<ref:my-heading>
```

renders as

<ref:a_piece_of_text>

<ref:heading>

