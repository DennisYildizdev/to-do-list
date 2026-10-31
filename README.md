# One more step

A calm to-do list that works on phone screens. Your next open task is shown large at the top. Everything else waits quietly below it.

It's written in Rust and compiled to WebAssembly. It started as a first GitHub project for learning HTML, CSS, JavaScript and the basic Git workflow, and was later rewritten in Rust.

## License

This project is released under the [MIT license](LICENSE).

## Use

Type a task and press **Add** or Enter. **Mark as done** completes the task at the top, and the next one moves up. To complete or reopen any other task, use its circle. The trash button deletes a task.

Browsers don't load WebAssembly from a local file, so double-clicking `index.html` won't work. Serve the folder instead:

```bash
python -m http.server 8000
```

Then open http://localhost:8000. GitHub Pages works too.

## Features

- Add, complete, reopen and delete tasks.
- The next open task stays in focus. Completed tasks move to the end of the list.
- Rejects empty tasks. Works with the keyboard and shows visible focus rings.
- Supports Turkish characters, narrow screens, long task text and dark mode.
- Saves in the browser and gives a clear warning when saving fails. A save it can't read is never overwritten.
- Reads lists saved by the earlier JavaScript version.
- No account and no server.

Tasks are stored only in the browser you use. They are never sent to GitHub. Clearing browser data deletes them. They don't sync across devices. Editing the same list in several tabs at once isn't supported.

## Project structure

- `index.html`: page structure and accessible labels.
- `style.css`: appearance, dark mode and mobile layout.
- `src/tasks.rs`: the task model, saved-data checks and unit tests. It has no browser code.
- `src/lib.rs`: DOM rendering, events and localStorage.
- `pkg/`: the compiled WebAssembly and its JavaScript glue. It's committed so GitHub Pages can serve it.
- `build.ps1`: builds `pkg/`.
- `.nojekyll`: lets GitHub Pages serve the files as they are.

## Build

One-time setup:

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cargo install wasm-bindgen-cli --version 0.2.130 --locked
```

Build, in PowerShell:

```bash
./build.ps1
```

Or run the two steps by hand in any shell:

```bash
cargo build --release --target wasm32-unknown-unknown
```

```bash
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/release/yapilacaklar.wasm
```

Run the tests:

```bash
cargo test
```

The `wasm-bindgen` CLI version must match the `wasm-bindgen` version in `Cargo.toml`.

## Learning with GitHub Desktop

1. Choose **File → Add local repository** and pick this folder.
2. If it isn't a Git repository yet, use **create a repository here**.
3. Review your changes, write a short summary and choose **Commit to main**.
4. In **Publish repository**, name it `yapilacaklar`, clear **Keep this code private** and publish.
5. After that, commit first and then use **Push origin** for each change.

A **commit** is a described snapshot of your changes, saved on your computer. A **push** sends those commits to GitHub. A saved file doesn't change on GitHub until you commit and push it.

## Publishing with GitHub Pages

In the repository, open **Settings → Pages → Build and deployment**. Choose **Deploy from a branch** as the source, **main** as the branch and **/(root)** as the folder, then save. When it finishes publishing, open the site address shown in the same section. Add that address to this README and to the repository's **About → Website** field.

Then change the footer sentence in `index.html`, commit and **Push origin**. Once Pages has published again, check the new text on the site. After a change to the Rust code, rebuild `pkg/` before you commit.

## Checklist

- [ ] The Add button and Enter both add a task.
- [ ] A task made only of spaces is rejected.
- [ ] Mark as done, reopen and delete all work.
- [ ] Tasks and their completed state survive a reload.
- [ ] Turkish characters and long text display correctly.
- [ ] Tasks can be managed with Tab, Enter and Space.
- [ ] There's no horizontal scrolling at phone width.
- [ ] Everything works on the published site too.
- [ ] After a commit and push, a text change appears on the live site.

Official guides: [GitHub Desktop](https://docs.github.com/en/desktop/overview/creating-your-first-repository-using-github-desktop) · [GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site)
