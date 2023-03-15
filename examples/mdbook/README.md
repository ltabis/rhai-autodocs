# mdbook

## Create a new book

```sh
cargo install mdbook
cargo init name-of-my-book
```

Add the default css to the book in the `book.toml` file.

```toml
additional-css = ["rhai-autodocs/styles/default.css"]
```

## Test and build

```sh
# To test and serve the book locally.
mdbook serve --open

# To publish the book.
mdbook build
```
