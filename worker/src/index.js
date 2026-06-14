const ROBLOX_TOKEN_URL = "https://apis.roblox.com/oauth/v1/token";
const ROBLOX_RESOURCES_URL = "https://apis.roblox.com/oauth/v1/token/resources";
const ROBLOX_REVOKE_URL = "https://apis.roblox.com/oauth/v1/token/revoke";

export default {
  async fetch(request, env) {
    if (request.method !== "POST") {
      return new Response("not found", { status: 404 });
    }

    const url = new URL(request.url);

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
