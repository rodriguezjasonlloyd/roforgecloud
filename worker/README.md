# roforgecloud-oauth-relay

A tiny Cloudflare Worker that proxies the Roblox OAuth2 token endpoints (`/oauth/v1/token`, `/oauth/v1/token/resources`, `/oauth/v1/token/revoke`), injecting this Worker's `OAUTH_CLIENT_ID`/`OAUTH_CLIENT_SECRET` instead of whatever the caller sends.

This lets roforgecloud perform the OAuth code/refresh-token exchange, revocation, and universe-discovery lookup without ever holding the real client secret — the secret lives only in this Worker's config.

## Deploy

```sh
npm install -g wrangler
wrangler login
wrangler secret put OAUTH_CLIENT_SECRET
wrangler deploy
```
