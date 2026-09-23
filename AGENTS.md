# Agent Guide

Score Shelf is an Ahara-integrated private shelf of versioned composition
exports: a Rust Lambda API, a phone-first React SPA, PostgreSQL metadata and
a private S3 bucket.

## Read first

| Topic                  | Link                                                                             |
| ---------------------- | -------------------------------------------------------------------------------- |
| Workspace overview     | [README.md](README.md)                                                           |
| Implementation plan    | [SCORE-SHELF-PLAN.md](SCORE-SHELF-PLAN.md)                                       |
| Documentation index    | [docs/README.md](docs/README.md)                                                 |
| Architecture           | [docs/architecture.md](docs/architecture.md)                                     |
| Architecture decisions | [docs/adr/README.md](docs/adr/README.md)                                         |
| Changelog              | [CHANGELOG.md](CHANGELOG.md)                                                     |
| Platform integration   | [../ahara/INTEGRATION.md](../ahara/INTEGRATION.md)                               |
| Ahara standards        | [../ahara-standards/standards/README.md](../ahara-standards/standards/README.md) |

## Critical rules

- Follow the Ahara platform contract: shared ALB, VPC, PostgreSQL/RDS, Cognito, Terraform state, and `ahara-tf-patterns` modules.
- File bytes never pass through the API. The ALB WAF and Lambda payload limits forbid it; uploads and downloads use presigned S3 URLs.
- The ALB verifies token signatures only. The API decides who the caller is from `client_id`, `token_use` and `scope`; never accept another app's tokens.
- A version is invisible until committed, and a commit checks every uploaded object's size against what was declared.
- The publisher client, its resource server and SSM parameters live in `ahara-infra` (`services/score-shelf-publisher.tf`), not here.
- Start local development servers only when the user explicitly asks.
- Run `make ci` before handoff after changing files.
- Use the secret broker only for commands that need injected secrets (`scripts/shelf.sh`, AWS, database).

## Code map

| Path                        | Purpose                                                           |
| --------------------------- | ----------------------------------------------------------------- |
| `backend/api/`              | Lambda: routing, auth, validation, PostgreSQL store, S3 presigning |
| `frontend/`                 | Vite React SPA: sign-in, pieces, versions, downloads, uploads      |
| `db/migrations/`            | PostgreSQL migrations for the `score_shelf` database               |
| `infrastructure/terraform/` | Project Terraform root using Ahara platform modules                |
| `scripts/`                  | `deploy.sh` and the `shelf.sh` publisher client                    |
| `docs/`                     | Architecture and ADRs                                              |

## Commands

| Command        | Purpose                                                   |
| -------------- | --------------------------------------------------------- |
| `make ci`      | Canonical local verification                              |
| `make db-test` | PostgreSQL store tests against a container                |
| `make build`   | Build the Rust workspace and the frontend                 |
| `make deploy`  | Run the parameterless local deploy script                 |

`make ci` enforces Clippy `-D warnings`, cognitive complexity `10`, function
length `75`, Rust and TypeScript files under `400` lines.
