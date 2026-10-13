# textual-items++

A keyboard-driven todo app for the terminal, inspired by Things 3 and Todoist, built in Rust with [Ratatui](https://ratatui.rs).

## Layout

- `app/` — the application (Rust crate: source code, tests, `scripts/`)
- `specs/` — requirements, specs and references ([index](specs/README.md))
- `docs/` — how agents work: workflow, version control, testing and code quality ([index](docs/README.md))

Agents start with [AGENTS.md](AGENTS.md). Work is tracked in the Todoist project `textual-items++`.

## Running

```sh
cd app
cargo run
```

Validation: `bash app/scripts/validate.sh all` from the repository root (`doctor` first checks the tools it needs).

## License

MIT, see [LICENSE](LICENSE).
