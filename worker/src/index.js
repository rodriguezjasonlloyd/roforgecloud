const ROBLOX_TOKEN_URL = "https://apis.roblox.com/oauth/v1/token";
const ROBLOX_RESOURCES_URL = "https://apis.roblox.com/oauth/v1/token/resources";
const ROBLOX_REVOKE_URL = "https://apis.roblox.com/oauth/v1/token/revoke";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (request.method === "GET" || request.method === "HEAD") {
      if (url.pathname === "/") {
        return html(ENTRY_PAGE);
      }
      if (url.pathname === "/privacy") {
        return html(PRIVACY_PAGE);
      }
      if (url.pathname === "/terms") {
        return html(TERMS_PAGE);
      }
      return new Response("not found", { status: 404 });
    }

    if (request.method !== "POST") {
      return new Response("not found", { status: 404 });
    }

    if (url.pathname === "/oauth/v1/token") {
      return proxy(ROBLOX_TOKEN_URL, request, env);
    }

    if (url.pathname === "/oauth/v1/token/resources") {
      return proxy(ROBLOX_RESOURCES_URL, request, env);
    }

    if (url.pathname === "/oauth/v1/token/revoke") {
      return proxy(ROBLOX_REVOKE_URL, request, env);
    }

    return new Response("not found", { status: 404 });
  },
};

function html(body) {
  return new Response(body, {
    headers: { "Content-Type": "text/html; charset=utf-8" },
  });
}

const ENTRY_PAGE = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>roforgecloud</title>
</head>
<body>
  <h1>roforgecloud</h1>
  <p>
    roforgecloud is an open-source CLI and terminal UI for browsing and
    managing Roblox Open Cloud resources (Data Stores, Messaging) under
    your own account via OAuth2.
  </p>
  <p>
    Source code: <a href="https://github.com/rodriguezjasonlloyd/roforgecloud">github.com/rodriguezjasonlloyd/roforgecloud</a>
  </p>
  <p>
    <a href="/privacy">Privacy Policy</a> &middot; <a href="/terms">Terms of Service</a>
  </p>
</body>
</html>`;

const PRIVACY_PAGE = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>roforgecloud - Privacy Policy</title>
</head>
<body>
  <h1>Privacy Policy</h1>
  <p>
    roforgecloud is a tool you run on your own computer. It uses Roblox's
    OAuth2 flow to obtain an access token scoped to the Roblox resources you
    explicitly authorize, and uses that token to call Roblox's Open Cloud
    APIs directly on your behalf.
  </p>
  <p>
    This site is operated as a small Cloudflare Worker ("the relay") that
    forwards OAuth token-exchange, token-refresh, token-revocation, and
    resource-lookup requests between roforgecloud and Roblox's OAuth
    servers, attaching this app's client credentials. The relay does not
    log, store, or share your access tokens, refresh tokens, or any data
    returned by Roblox's APIs &mdash; requests are forwarded and the
    responses are returned directly to you.
  </p>
  <p>
    No personal data is collected, stored, or sold by roforgecloud or this
    relay.
  </p>
</body>
</html>`;

const TERMS_PAGE = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>roforgecloud - Terms of Service</title>
</head>
<body>
  <h1>Terms of Service</h1>
  <p>
    roforgecloud and this OAuth relay are provided "as is", without
    warranty of any kind, express or implied. Use of roforgecloud is
    subject to Roblox's own Terms of Use and API/Open Cloud terms, which
    govern what you may do with your Roblox account and data.
  </p>
  <p>
    You are responsible for any actions taken against your Roblox
    resources (Data Stores, Messaging, etc.) using the access you grant
    via OAuth, including destructive operations such as deleting or
    modifying data.
  </p>
  <p>
    The source code for roforgecloud and this relay is publicly available
    and may be self-hosted under its open-source license.
  </p>
</body>
</html>`;

async function proxy(targetUrl, request, env) {
  const incoming = await request.formData();
  const body = new URLSearchParams();
  for (const [key, value] of incoming.entries()) {
    if (key === "client_id" || key === "client_secret") continue;
    body.set(key, value);
  }
  body.set("client_id", env.OAUTH_CLIENT_ID);
  body.set("client_secret", env.OAUTH_CLIENT_SECRET);

  const response = await fetch(targetUrl, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: body.toString(),
  });

  return new Response(response.body, {
    status: response.status,
    headers: {
      "Content-Type": response.headers.get("Content-Type") ?? "application/json",
    },
  });
}
