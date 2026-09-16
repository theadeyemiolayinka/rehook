---
layout: home

hero:
  name: Rehook
  text: Webhook capture and local delivery
  tagline: Receive webhooks on a public server while your local machine is offline. Deliver them later through an authenticated agent.
  image:
    src: /logo.svg
    alt: Rehook
  actions:
    - theme: brand
      text: Get Started
      link: /getting-started/server
    - theme: alt
      text: Agent/Client Setup
      link: /getting-started/agent
    - theme: alt
      text: GitHub
      link: https://github.com/theadeyemiolayinka/rehook

features:
  - title: Capture and store
    details: Public server receives webhooks at unguessable URLs. Events are stored in SQLite. Unknown endpoints respond identically to avoid leaking existence.
  - title: Local delivery
    details: Agent connects outbound via WebSocket. Server sends only a target identifier, never a URL. Agent validates against a local allowlist before delivery.
  - title: Not a reverse tunnel
    details: The server cannot browse your local network. It can only request delivery to explicitly configured targets. The agent rejects unsupported schemes.
  - title: Two dashboards
    details: Admin dashboard for server management. Agent web UI for local target configuration, event inspection, and replay. Both bound to their respective contexts.
  - title: Self-hostable
    details: Single binary server. SQLite storage. No Redis, no Kafka, no cloud dependencies. Deploy with Docker Compose, Coolify, or Dokploy.
  - title: Security first
    details: Argon2id passwords. HTTP-only session cookies. Sensitive header masking. Hop-by-hop filtering. No response body leakage to the server.
---
