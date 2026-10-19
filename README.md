# topcoat-taskboard

A live team task board built with [Topcoat](https://github.com/tokio-rs/topcoat) 0.10 and [Toasty](https://github.com/tokio-rs/toasty) 0.11. It is the companion project for the LogRocket article *Building with Topcoat, the Rust framework that skips WebAssembly entirely*.

The board shows tasks in four columns. Each feature uses a different part of Topcoat:

| Feature | Topcoat mechanism | Where |
|---|---|---|
| Hide done | Signal, `@click`, and a `:class` binding, with no network request | `home` in `src/main.rs` |
| Search | `#[shard]` re-rendered on the server as you type | `search_results` in `src/main.rs` |
| Move a task | `#[procedure]` that validates every move on the server | `move_task` in `src/main.rs` |
| Live updates | `live!` and `emit!` in a shard, pushed over a WebSocket to every open tab | `board` in `src/main.rs` |

The `/leaks` page reproduces two Rust and JavaScript mismatches from Topcoat's own known-mismatch list, each next to its fix. It lives in `src/leaks.rs`.

## Requirements

- Rust 1.98 or later
- The Topcoat CLI at the matching version:

```sh
cargo install topcoat-cli --version 0.10.0 --locked
```

## Run it

```sh
git clone https://github.com/Ikeh-Akinyemi/topcoat-taskboard.git
cd topcoat-taskboard
topcoat dev
```

Open <http://127.0.0.1:3000> for the board and <http://127.0.0.1:3000/leaks> for the mismatch demo. Use `topcoat dev` rather than `cargo run`, because the CLI builds the asset bundle that serves Topcoat's browser runtime.

The board uses an in-memory SQLite database seeded on startup, so every restart resets it.

## Layout

| Path | Contents |
|---|---|
| `src/main.rs` | Router setup, layout, board page, search shard, live board shard, and the `move_task` procedure |
| `src/tasks.rs` | The `Task` model, the `Column` enum, seed data, and the broadcaster that notifies connected boards |
| `src/leaks.rs` | The `/leaks` page with the early-return and captured-NaN mismatches |

Tested with topcoat 0.10.0, toasty 0.11.0, and Rust 1.99.1.
