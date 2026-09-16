# Agent Targets

Targets are the local HTTP destinations the agent can deliver webhooks to.

## How targets work

1. The server sends a delivery instruction containing a `target_id` (a string identifier, not a URL).
2. The agent looks up the `target_id` in its local configuration.
3. The agent validates the URL scheme (only `http` and `https` are allowed).
4. The agent reconstructs the request, filters hop-by-hop headers, and sends it to the local target.
5. The agent records the result locally.

The server never sends a URL. The agent never fetches a URL that is not in its configuration.

## Adding a target

From the web UI (Targets page):

1. Enter a target ID (e.g. `myapp`).
2. Enter the local URL (e.g. `http://localhost:8000/webhook`).
3. Click Add.

From the terminal:

```bash
rehook target add myapp http://localhost:8000/webhook
```

## Allowed destinations

- `http://localhost:...`
- `http://127.0.0.1:...`
- `https://localhost:...`
- Laravel Valet `.test` domains (e.g. `http://myapp.test/webhook`)
- Any explicit `http://` or `https://` URL you configure

## Forbidden schemes

The agent rejects these schemes:
- `file:`
- `ftp:`
- `gopher:`
- `data:`
- `javascript:`
- `ws:` and `wss:`

## Routes

A route maps an endpoint to a target. When the server sends a delivery instruction for an event on that endpoint, the agent uses the route to determine which target to deliver to.

Different endpoints in the same project can route to different targets. For example, in a "payments" project, you can route the "paystack" endpoint to one target and the "stripe" endpoint to another.

From the web UI, go to the Routes page and click New route. Pick an endpoint from the dropdown (grouped by project) and connect it to a target.

From the terminal:

```bash
rehook route add <endpoint-id> myapp
```

Get the endpoint ID from the admin dashboard (Endpoints page).

## Removing a target

From the web UI, click Remove next to the target.

From the terminal:

```bash
rehook target remove myapp
```

Removing a target also removes any routes that reference it.
