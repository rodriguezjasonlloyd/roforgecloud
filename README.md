# roforgecloud

A Rust companion for Roblox: a wrapper around the Open Cloud APIs and
OAuth2 flow, plus a CLI built on top of it.

## Workspace layout

- `crates/roforgecloud-core` — library crate
  - `opencloud` — Open Cloud REST client (datastores, messaging, ...)
  - `oauth` — Roblox OAuth2 / OIDC client (PKCE authorization code flow, refresh, token resources)
- `crates/roforgecloud-cli` — `roforgecloud` binary
- `crates/roforgecloud-tui` — `roforgecloud-tui` terminal browser for Open Cloud

## CLI usage

```sh
export ROFORGE_API_KEY=<API_KEY>

rofc datastore list-stores <universe_id>
rofc datastore get <universe_id> <data_store_id> <entry_id> [--scope <scope>]
rofc datastore set <universe_id> <data_store_id> <entry_id> '{"foo": "bar"}' [--scope <scope>]
rofc datastore delete <universe_id> <data_store_id> <entry_id> [--scope <scope>]
rofc datastore list <universe_id> <data_store_id> [--filter 'id.startsWith("foo")'] [--scope <scope>]
rofc datastore list-scopes <universe_id> <data_store_id>
rofc messaging publish <universe_id> <topic> <message>
```

## TUI usage

```sh
export ROFORGE_API_KEY=<API_KEY>
rofct
```

The TUI opens to a menu (Data Stores, Messaging). Selecting an item then
asks whether to enter a universe ID by hand or list authorized universes.

### OAuth mode

In addition to an API key, the TUI authenticates via OAuth2 by default — no
setup required. It ships with a built-in `client_id` and talks to a hosted
relay (see `worker/`) that holds the matching client secret, so there's
nothing to configure:

```sh
roforgecloud-tui
```

If `ROFORGE_API_KEY` is also set, the API key is used for Open Cloud calls,
and OAuth is only used for the "list my universes" option below.

Selecting "List my universes (OAuth)" (or running with no API key at all)
triggers the OAuth flow on first use: it prints an authorization URL (and
opens it in your browser), then listens on the redirect URI for the
callback. The resulting token (and refresh token) are cached at
`~/.config/roforgecloud/token.json`. It then calls the `token/resources`
endpoint (via the relay) to discover which universes the token was granted
access to, and offers them as a selectable list.

Open Cloud DataStore access is not currently available via OAuth2 — only
`universe-messaging-service:publish` (and `universe:read`, used to discover
authorized universes) are requested. If you're relying solely on OAuth (no
API key), Data Stores calls will fail; use an API key for DataStore access.

To use your own OAuth app instead of the built-in one, set
`ROFORGE_OAUTH_CLIENT_ID` and `ROFORGE_OAUTH_CLIENT_SECRET` — this talks to
Roblox directly and bypasses the relay. `ROFORGE_OAUTH_REDIRECT_URI` defaults
to `http://localhost:8675/callback`. `ROFORGE_OAUTH_RELAY_URL` overrides the
relay URL (ignored if `ROFORGE_OAUTH_CLIENT_SECRET` is set).
