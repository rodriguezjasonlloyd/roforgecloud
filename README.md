<p align="center">
  <img src="logo_text.svg" alt="roforgecloud" width="400">
</p>

A tool for browsing and managing your Roblox game data and sending messages to running game servers, without having to write API calls by hand.

It comes in two forms:

- **`rofct`** — a terminal app (TUI) you can navigate with arrow keys. This is the easiest way to use it.
- **`rofc`** — a command-line tool (CLI) for scripting/automation.

## Getting started (TUI)

```sh
rofct
```

That's it — no setup required. It'll open a browser tab asking you to log in with your Roblox account and approve access. After that, you'll see a menu:

- **Data Stores** — browse, search, view, edit, and delete entries in your game's data stores.
- **Messaging** — publish a message to a topic, for live communication with running game servers.

Pick one, then either type in a universe (game) ID directly, or choose "list my universes" to pick from the games your Roblox account has access to.

Login only happens once — your session is cached at `~/.config/roforgecloud/token.json` and refreshed automatically. Run `rofct --logout` to sign out.

### Data Store access

Browsing/editing Data Store entries requires an [Open Cloud API key](https://create.roblox.com/docs/cloud/auth/api-keys) for that game, set via:

```sh
export ROFORGE_API_KEY=<your API key>
```

Without an API key, the Messaging and "list my universes" features still work via the browser login above, but Data Store access will fail — Roblox doesn't currently allow Data Store access through that login method.

## CLI usage

```sh
export ROFORGE_API_KEY=<your API key>

rofc datastore list-stores <universe_id>
rofc datastore get <universe_id> <data_store_id> <entry_id> [--scope <scope>]
rofc datastore set <universe_id> <data_store_id> <entry_id> '{"foo": "bar"}' [--scope <scope>]
rofc datastore delete <universe_id> <data_store_id> <entry_id> [--scope <scope>]
rofc datastore list <universe_id> <data_store_id> [--filter 'id.startsWith("foo")'] [--scope <scope>]
rofc datastore list-scopes <universe_id> <data_store_id>
rofc messaging publish <universe_id> <topic> <message>
```

## Workspace layout

- `crates/roforgecloud-core` — shared library (Open Cloud API client, login)
- `crates/roforgecloud-cli` — the `rofc` binary
- `crates/roforgecloud-tui` — the `rofct` binary
- `worker/` — small Cloudflare Worker that lets the browser login work without you needing to register your own app with Roblox

## Advanced: bringing your own login app

By default, the browser login uses a shared app registered by roforgecloud, via the relay in `worker/`. If you'd rather register your own app with Roblox, set:

```sh
export ROFORGE_OAUTH_CLIENT_ID=<your client id>
export ROFORGE_OAUTH_CLIENT_SECRET=<your client secret>
```

This bypasses the relay and talks to Roblox directly.
`ROFORGE_OAUTH_REDIRECT_URI` (defaults to `http://localhost:8675/callback`) must match a redirect URI registered on your app.
`ROFORGE_OAUTH_RELAY_URL` lets you point at your own deployed relay instead (ignored if a client secret is set).
