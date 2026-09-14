# Agent Web UI

The agent web UI is a local browser interface for configuring and managing the agent. You can do everything from the web UI without using the terminal.

## Starting the web UI

```bash
hookrelay web
```

Open `http://localhost:8787` in your browser. The web UI runs locally on your machine.

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

Click Connect. The agent validates the server and stores your credentials.

## What you can do

### Connection page

View your server URL, agent identity, and subscribed projects. Sign out from here.

### Targets and Routes page

Add and remove local delivery targets. Add and remove project-to-target routes. When you add a route, you select a target from a dropdown of your configured targets.

### Events page

Inspect captured webhook events stored locally. Click an event to see full headers and payload. Replay any event to a configured target.

### History page

View delivery attempts with HTTP status, duration, and error details.

### Settings page

View local database stats. Clear the local database (requires confirmation). Sign out.

## Local replay

From the Events page, click Replay on any event. Select a target. The agent delivers the webhook to that target and records the result. The result appears on the History page.

## Relationship to the terminal

The web UI and the terminal share the same configuration. Changes made in the web UI are visible to the terminal and vice versa.

The `hookrelay start` command manages the live connection to the server. The web UI does not start or stop this connection. Typical workflow:

1. Run `hookrelay web` to configure the agent in the browser.
2. Run `hookrelay start` in a terminal to connect to the server.
3. Use the web UI to inspect events and trigger local replays.
