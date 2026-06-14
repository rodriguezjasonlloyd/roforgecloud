# roforgecloud

![roforgecloud](logo_text.svg)

Browse and manage your Roblox game's Data Stores and send live messages to
game servers without writing any API calls yourself.

It comes in two forms:

- **`rofct`** — a terminal app you navigate with arrow keys. Start here.
- **`rofc`** — a command-line tool for scripting/automation, if you need it.

## Installation

### Option 1: mise (preferred)

If you use [mise](https://mise.jdx.dev/):

```sh
mise use "github:rodriguezjasonlloyd/roforgecloud"
```

This grabs the latest prebuilt release for your platform.

### Option 2: download a release

Grab the archive for your platform from the
[latest release](https://github.com/rodriguezjasonlloyd/roforgecloud/releases/latest),
extract it, and put `rofct` (and `rofc`, if you want it) somewhere on your
`PATH`.

### Option 3: build from source

You'll need [Rust installed](https://www.rust-lang.org/tools/install) (via
`rustup`). Then:

```sh
cargo install --git https://github.com/rodriguezjasonlloyd/roforgecloud roforgecloud-tui
```

This installs the `rofct` command. (If you also want the scripting tool,
`cargo install --git https://github.com/rodriguezjasonlloyd/roforgecloud roforgecloud-cli`
installs `rofc`.)

## Using the terminal app (`rofct`)

```sh
rofct
```

It should open a browser tab asking you to log in with your
Roblox account and approve access.

- **Data Stores** — browse, search, view, edit, and delete entries in your
  game's data stores.
- **Messaging** — publish a message to a topic, for live communication with
  running game servers.

Pick one, then either type in a universe (game) ID directly, or choose "list
my universes" to pick from the games your Roblox account has access to.

You only have to log in once — your session is saved to
`~/.config/roforgecloud/token.json` and refreshed automatically. The main
menu has a "Login"/"Logout" entry if you want to switch accounts or sign out.

### Editing Data Store entries needs an API key

The browser login above is enough for Messaging and for listing your games,
but **viewing/editing Data Store entries requires an [Open Cloud API
key](https://create.roblox.com/docs/cloud/auth/api-keys)** for that game.

Create an API key for your game (with Data Store read/write permissions),
then run:

```sh
export ROFORGE_API_KEY=<your API key>
rofct
```

## Using the CLI (`rofc`)

```sh
export ROFORGE_API_KEY=<your API key>

rofc datastore list-stores <universe_id>
rofc datastore get <universe_id> <data_store_id> <entry_id> [--scope <scope>]
rofc datastore set <universe_id> <data_store_id> <entry_id> '{"foo": "bar"}' [--scope <scope>]
rofc datastore delete <universe_id> <data_store_id> <entry_id> [--scope <scope>]
rofc datastore list <universe_id> <data_store_id> [--filter 'id.startsWith("foo")'] [--scope <scope>]
rofc datastore list-scopes <universe_id> <data_store_id>
rofc messaging publish <universe_id> <topic> <message>
rofc login
rofc logout
```

(`<universe_id>` is your game's ID, found in the Creator Dashboard URL.)

---

## Project layout

- `crates/roforgecloud-core` — shared library (Open Cloud API client, login)
- `crates/roforgecloud-cli` — the `rofc` binary
- `crates/roforgecloud-tui` — the `rofct` binary
- `worker/` — small Cloudflare Worker that lets the browser login work
  without you needing to register your own Roblox OAuth app

## Advanced: bringing your own login app

By default, the browser login uses a shared app registered by roforgecloud,
via the relay in `worker/`. If you'd rather register your own app with
Roblox, set:

```sh
export ROFORGE_OAUTH_CLIENT_ID=<your client id>
export ROFORGE_OAUTH_CLIENT_SECRET=<your client secret>
```

This bypasses the relay and talks to Roblox directly.
`ROFORGE_OAUTH_REDIRECT_URI` (defaults to `http://localhost:8675/callback`)
must match a redirect URI registered on your app.
`ROFORGE_OAUTH_RELAY_URL` lets you point at your own deployed relay instead
(ignored if a client secret is set).
