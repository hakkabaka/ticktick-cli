# ticktick-cli

Terminal client for TickTick with OAuth login and a `ratatui` interface.

## Features

- OAuth authentication with TickTick
- Project list screen (`ProjectsView`)
- Ticket/task list per selected project (`ProjectView`)
- Keyboard navigation in terminal UI

## Requirements

- Rust (stable)
- TickTick Open API app credentials:
  - `TICKTICK_CLIENT_ID`
  - `TICKTICK_CLIENT_SECRET`

## Setup

Set environment variables in your shell (You can register your application by visiting the [TickTick Developer Center](https://developer.ticktick.com/manage)):

```bash
export TICKTICK_CLIENT_ID="your_client_id"
export TICKTICK_CLIENT_SECRET="your_client_secret"
# optional, default is http://127.0.0.1:8080/callback
export TICKTICK_REDIRECT_URI="http://127.0.0.1:8080/callback"
```

Important: variables must be `export`ed so `cargo run` can pass them to the app process.

## Run

```bash
cargo run
```

On first run, the app opens the OAuth browser flow and then starts the TUI.

## Project Structure

- `src/main.rs`: app bootstrap, env loading, API bootstrap
- `src/ticktick.rs`: TickTick API client + response models
- `src/app.rs`: app state and screen transitions
- `src/ui.rs`: `ratatui` rendering + keyboard event handling
- `src/oauth.rs`: OAuth PKCE implementation
