# myownredis

A small Redis clone in Rust.

- `resp` – RESP protocol encoder/decoder
- `server` – async server on `127.0.0.1:6379`
- `cli` – command-line client

Supported commands: `PING`, `ECHO`, `SET` (with `EX`/`PX`), `GET`, `DEL`, `EXISTS`.

## Usage

```sh
cargo run -p server
```

```sh
cargo run -p cli                  # interactive
cargo run -p cli -- set foo bar   # one-shot
cargo run -p cli -- -h 127.0.0.1 -p 6379 get foo
```

Log level is set with `RUST_LOG` (default `info`).
