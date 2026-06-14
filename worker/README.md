# roforgecloud-oauth-relay

A tiny Cloudflare Worker that proxies the Roblox OAuth2 token endpoints
(`/oauth/v1/token` and `/oauth/v1/token/resources`), injecting this Worker's
`OAUTH_CLIENT_ID`/`OAUTH_CLIENT_SECRET` instead of whatever the caller sends.

This lets the roforgecloud CLI perform the OAuth code/refresh-token exchange
and universe-discovery lookup without ever holding the real client secret —
the secret lives only in this Worker's config.

## Deploy

```sh
npm install -g wrangler
wrangler login

# set the OAuth app's client id (not secret, fine to commit)
# edit wrangler.toml: vars.OAUTH_CLIENT_ID = "<your client id>"

# set the OAuth app's client secret
wrangler secret put OAUTH_CLIENT_SECRET

wrangler deploy
```

This gives you a `https://roforgecloud-oauth-relay.<your-subdomain>.workers.dev`
URL. Pointing roforgecloud's `TOKEN_URL`/`RESOURCES_URL` at this instead of
`https://apis.roblox.com/...` is not wired up yet — that's a follow-up change
to `crates/roforgecloud-core/src/oauth/mod.rs` to make those URLs
configurable.
