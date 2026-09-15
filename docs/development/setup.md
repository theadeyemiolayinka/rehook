# Development Setup

## Prerequisites

- Rust 1.80 or later (install via [rustup](https://rustup.rs))
- Node.js 22 or later
- Docker (optional, for container builds)

## Building from source

```bash
# Clone the repository
git clone https://github.com/theadeyemiolayinka/hookrelay.git
cd hookrelay

# Build all Rust crates
cargo build

# Build the admin dashboard
cd dashboards/admin
npm install
npm run build

# Build the agent dashboard
cd ../agent
npm install
npm run build
```

## Running in development

Start the server:

```bash
cp .env.example .env
# Edit .env: set ADMIN_PASSWORD and HOOKRELAY_SESSION_KEY
cargo run -p hookrelay-server
```

Start the admin dashboard dev server (with API proxy to the backend):

```bash
cd dashboards/admin
npm run dev
```

Open `http://localhost:5173` for the dashboard. The API is proxied to `http://localhost:8080`.

Start the agent dashboard dev server:

```bash
cd dashboards/agent
npm run dev
```

Open `http://localhost:5174`. The API is proxied to `http://localhost:8787`.

## Running the agent

```bash
cargo run -p hookrelay-agent -- login --server http://localhost:8080 --agent-id <id> --token <token>
cargo run -p hookrelay-agent -- target add myapp http://localhost:8000/webhook
cargo run -p hookrelay-agent -- route add <endpoint-id> myapp
cargo run -p hookrelay-agent -- web
```

Open `http://localhost:8787`. `hookrelay web` starts both the web UI and the connection loop.

## Running the docs site

```bash
cd docs
npm install
npm run dev
```

Open `http://localhost:5173`. The docs site rebuilds on file changes.

To build the docs for production:

```bash
cd docs
npm run build
```

Output is in `docs/.vitepress/dist`.

## Repository structure

```
hookrelay/
├── apps/server/       Axum HTTP server
├── apps/agent/        Rust CLI agent
├── crates/protocol/   Shared WebSocket protocol
├── dashboards/admin/  Admin dashboard (React)
├── dashboards/agent/  Agent web UI (React)
├── dashboards/shared/ Shared design system
├── docs/              Documentation
├── scripts/           Build scripts
├── Dockerfile         Multi-stage production image
└── compose.yml        Docker Compose configuration
```
