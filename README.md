# fafnir-wallet

> HTTP-served SSI wallet for the EUNOMIA dataspace.

`fafnir-wallet` is a minimal Rust wallet that runs as a **separate service**
next to an SSI agent (`heimdall`, `ds-agent`, or any other ymir consumer).
Agents talk to it over HTTP through the `FafnirService` adapter shipped in
`ymir`, which implements `WalletTrait`.

It custodies private keys, signs JWTs, and speaks the holder side of
**OID4VCI** and **OID4VP**. It is not a user-facing wallet — it has no UI,
no login, no session. Think of it as a sidecar daemon for an SSI node.

## Status

**Feature-complete for the wallet mode.** The current API covers everything
`heimdall` and `ds-agent` need today (keys, DIDs, VCs, OID4VCI holder, OID4VP
holder). The repo will stay frozen until the next planned milestone:

> **Next milestone**: support a second mode of operation as **Credential
> Service**, following the Decentralized Claims Protocol (DCP) used by the
> Eclipse Dataspace Components. The wallet will then expose both the current
> "holder wallet" surface and the DCP credential-service surface.

Until that work starts, expect no API changes here.

## How it runs

Canonical deployment is **a container next to the agent** in the same
`docker-compose`. The image is fully self-contained: the only YAML it needs
(`static/config/fafnir-docker.yaml`) is baked in at build time.

```bash
docker build -t fafnir-wallet:dev .
docker run --rm -p 7001:7001 fafnir-wallet:dev
```

In a real deployment, the wallet is a service inside the same compose as the
agent that consumes it:

```yaml
services:
  fafnir-wallet:
    image: fafnir-wallet:dev
    expose: ["7001"]
    depends_on: [fafnir-db]

  fafnir-db:
    image: postgres:16
    environment:
      POSTGRES_USER: ...
      POSTGRES_PASSWORD: ...
      POSTGRES_DB: ...
```

The wallet container runs as `appuser` (UID 10001) and exposes port `7001`.

## How agents talk to it

```
┌─────────────────────────┐         ┌───────────────────────┐
│  heimdall / ds-agent    │         │    fafnir-wallet      │
│                         │   HTTP  │                       │
│  ymir::WalletTrait      │ ──────► │  axum HTTP server     │
│    └─ FafnirService     │         │  on :7001             │
└─────────────────────────┘         └──────────┬────────────┘
                                               │ SeaORM
                                               ▼
                                       ┌───────────────┐
                                       │   Postgres    │
                                       └───────────────┘
```

In the agent's YAML you pick the wallet backend:

```yaml
wallet:
  wallet: Fafnir            # or WaltId
  api:
    protocol: 'http'
    url: 'fafnir-wallet'    # service name in docker-compose
    port: null
    internal_port: '7001'
```

The agent's `setup` code (`heimdall/src/core/builder.rs` /
`ds-agent/crates/auth/src/setup/app.rs`) constructs `FafnirService` and calls
`link()` once at startup. That call provisions the agent's identity in the
wallet: reads the PEM from Vault, creates a key and a DID (`did:jwk` or
`did:web`, depending on the agent's config), and marks the DID as default.

## Stack

- **Language**: Rust 2024 edition.
- **HTTP**: `axum` 0.8 + `tokio`.
- **Crypto**: `ed25519-dalek`, `rsa`, `sha2` — pure RustCrypto, no OpenSSL.
- **Persistence**: Postgres via SeaORM (`sqlx-postgres`). SQLite is feature-
  enabled but not the default target.
- **Config**: YAML via `serde_norway`.
- **Shared types**: pulls `WalletTrait`, `Signer`, `Verifier`, DID/key/VC
  models, etc. from `ymir`.

## Repository layout

```
src/
├── main.rs                # entrypoint
├── lib.rs
├── setup/                 # CLI (clap), app wiring, db_migrations
├── config/                # YAML parsing (CoreWalletConfig)
├── core/                  # core state
├── modules/               # orchestrators: keys, dids, vcs, oid4vc
├── http/                  # routers: keys, dids, vcs, oid4vc, core
├── services/
│   ├── repo/              # SeaORM-backed repository
│   ├── oid4vci/           # holder-side OID4VCI service
│   └── oid4vp/            # holder-side OID4VP service
├── data/                  # SeaORM entities + migrator
├── types/
└── utils/

static/config/fafnir-docker.yaml   # baked into the image at build time
Dockerfile                          # multi-stage, debian-slim final
```

## Configuration

The Docker image ships with this YAML built in:

```yaml
http_host: &http_host
  protocol: 'http'
  url: '127.0.0.1'
  port: null
  internal_port: '7001'

hosts:
  http: *http_host
  grpc: null
  graphql: null

db_config:
  db_type: Postgres
  url: 'heimdall-db'        # hostname of the DB container in the same compose
  port: '5432'

is_docker: true
is_vault_real: false
```

To override (different DB hostname, different port, etc.), mount your own
YAML into the same path:

```yaml
services:
  fafnir-wallet:
    volumes:
      - ./my-wallet.yaml:/app/static/config/fafnir-docker.yaml:ro
```

## HTTP API

### Keys

| Method | Path           | Body / Response                | Notes                       |
|--------|----------------|--------------------------------|-----------------------------|
| GET    | `/keys/all`    | → `Vec<KeyEntry>`              |                             |
| GET    | `/keys/{id}`   | → `KeyEntry` \| 404            |                             |
| POST   | `/keys/new`    | `KeyEntryReq` → `KeyEntry` 201 | Validates PKCS#8 on store   |
| DELETE | `/keys/{id}`   | → 204                          |                             |

### DIDs

| Method | Path                  | Body / Response                | Notes                                  |
|--------|-----------------------|--------------------------------|----------------------------------------|
| GET    | `/dids/all`           | → `Vec<DidEntry>`              |                                        |
| GET    | `/dids/default`       | → `DidEntry` \| 404            |                                        |
| GET    | `/dids/{id}`          | → `DidEntry` \| 404            |                                        |
| POST   | `/dids/new`           | `DidEntryReq` → `DidEntry` 201 | Supports `Jwk` and `Web`               |
| POST   | `/dids/{id}/default`  | → 200                          | Marks this one, clears the others      |
| DELETE | `/dids/{id}`          | → 204                          |                                        |

### VCs

| Method | Path         | Body / Response       | Notes                                |
|--------|--------------|-----------------------|--------------------------------------|
| GET    | `/vcs/all`   | → `Vec<VcEntry>`      |                                      |
| GET    | `/vcs/{id}`  | → `VcEntry` \| 404    |                                      |
| DELETE | `/vcs/{id}`  | → 204                 | VCs are issued via OID4VCI, not POST |

### OID4VC (one-shot)

What `FafnirService` calls in ymir:

| Method | Path        | Body                  | Response                       |
|--------|-------------|-----------------------|--------------------------------|
| POST   | `/oid4vci`  | `{ "uri": "..." }`    | 200 (VC stored)                |
| POST   | `/oid4vp`   | `{ "uri": "..." }`    | 200 + optional `redirect_uri`  |

### OID4VC (stepwise)

For consumers that want to show the offer / VP request / matching before
the user confirms:

| Method | Path                    | Body                | Response       |
|--------|-------------------------|---------------------|----------------|
| POST   | `/vci/resolve_offer`    | `{ "uri": "..." }`  | `VCCredOffer`  |
| POST   | `/vci/accept_offer`     | `VCCredOffer`       | 201            |
| POST   | `/vp/resolve_request`   | `{ "uri": "..." }`  | `VPDef`        |
| POST   | `/vp/present`           | `{ "uri": "..." }`  | `redirect_uri` |

## Security model

This is a **sidecar** wallet, not a public service. The deployment model
assumes:

- The wallet container is reachable **only from other containers in the
  same compose** (i.e. its port is not published to the host).
- Vault holds the bootstrap PEM; the wallet stores derived keys at rest
  in Postgres **without additional encryption**.
- No authentication on the HTTP API. Anyone who can route to `:7001` can
  use it.

If you expose `:7001` outside the compose network, you are using it wrong.

## Known limitations

The wallet mode is feature-complete for what `heimdall` and `ds-agent`
exercise. Known rough edges, deferred until a future iteration:

- No at-rest encryption for private keys in Postgres.
- `did:web` resolution picks the first `verificationMethod` — works for
  single-VM documents, not for multi-VM with split purposes.
- Only the first key bound to a DID is used for signing; no separation
  between `authentication` / `assertionMethod` / `keyAgreement`.
- `match_vcs` matches by type name only; doesn't evaluate
  `constraints.fields[*].filter.pattern` regex.
- `request_uri` in OID4VP only supports JWTs that carry the
  `presentation_definition` inline.
- No `c_nonce` enforcement in the OID4VCI proof.
- No OpenAPI definition, no automated test suite.

## Roadmap

| Phase            | Status     | Notes                                                 |
|------------------|------------|-------------------------------------------------------|
| Wallet mode      | **Done**   | Frozen API. Iteration paused.                         |
| Credential Service mode (DCP) | Planned | Add the Eclipse-style credential-service surface alongside wallet mode. |
| At-rest key encryption | Planned | Sealed with a Vault-managed wrap key.            |
| OpenAPI + integration tests | Planned | Needed before opening the API beyond EUNOMIA.|

## License

GPL-3.0-or-later. See the headers of each `.rs` file for the full notice.
