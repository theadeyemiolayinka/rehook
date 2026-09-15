# Agent Web UI

The agent web UI is a local browser interface for configuring and managing the agent. You can do everything from the web UI without using the terminal.

## Starting the web UI

```bash
hookrelay web
```

Open `http://localhost:8787` in your browser. The web UI runs locally on your machine.

`hookrelay web` starts both the web UI and the WebSocket connection to the server. You do not need to run a separate command.

To use a different port:

```bash
hookrelay web --port 9000
```

## First-time setup

If the agent is not yet configured, the web UI shows a login page. Enter the details from your server admin dashboard:

- **Server URL**: your HookRelay server URL
- **Agent ID**: the UUID from the admin dashboard
- **Agent Token**: the one-time token from the admin dashboard
- **Agent Name** (optional): a friendly name for this machine

Click Connect. The agent validates the server, stores your credentials, and connects automatically. The connection status updates on the Connection page.

## What you can do

### Connection page

View your server URL, agent identity, and the endpoints you are subscribed to. Sign out from here.

### Targets page

Add and remove local delivery targets. A target is a local HTTP destination (e.g. `http://localhost:8000/webhook`) identified by a short ID (e.g. `myapp`). The agent only ever delivers to URLs you configure here.

### Routes page

Connect inbound endpoints to local targets. A route maps an endpoint from your HookRelay server to a local target. When the server sends a delivery for an event on that endpoint, the agent uses the route to find the right target.

Different endpoints in the same project can route to different targets. For example, in a "payments" project, route the "paystack" endpoint to one target and the "stripe" endpoint to another.

The endpoint dropdown is grouped by project so you can identify endpoints by name rather than UUID.

### Events page

Inspect captured webhook events stored locally. Click an event to see full headers and payload on a dedicated detail page. Replay any event to a configured target.

### History page

View delivery attempts with HTTP status, duration, and error details. Click a delivery to jump to the related event.

### Settings page

View local database stats. Clear the local database (requires confirmation). Sign out.

## Local replay

From an event's detail page, click Replay. Select a target. The agent delivers the webhook to that target and records the result. The result appears on the History page.

## Relationship to the terminal

The web UI and the terminal share the same configuration. Changes made in the web UI are visible to the terminal and vice versa.

`hookrelay web` starts both the web UI and the connection loop. If you only want the connection loop without the web UI, use `hookrelay start` instead.
