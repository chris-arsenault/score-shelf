# Score Shelf plan

## Outcome and scope

Score Shelf is a private, versioned shelf for composition exports (MusicXML
and MIDI). An agent on the dev box publishes each new export as a labelled
version of a piece. The owner signs in on a phone, browses pieces and their
versions, downloads files, and uploads hand-edited MusicXML back as an
owner version for the agent to import.

In scope: the score-shelf project on the Ahara platform, its registration in
`ahara-infra`, the publisher's machine-to-machine client, the listener
priority registry row in `ahara`, and a publish script that sigillum
exports can call. Out of scope: in-browser score rendering and MIDI playback
(a later phase), sharing with anyone but the owner (that belongs to
`ahara-access`), and deleting versions.

Authorization: the owner asked to build it and wire it into the platform,
following existing standards (2026-09-23). Pushing `ahara-infra` main applies
the platform control and services layers through CI.

## Context and reuse map

| Need | Source followed |
| --- | --- |
| Platform integration steps | `ahara/INTEGRATION.md` Steps 1–10, `ahara/CI-WORKFLOW.md` |
| Project template | `bookmarker` (linkdrop): layout, Makefile, CI, Terraform, `lambda_http` + telemetry, private bucket, trait-based test doubles |
| Private file storage decision | `bookmarker/docs/adr/0003-project-owned-snapshot-storage.md`: a single-owner private library keeps a project bucket plus Postgres metadata; `ahara-access` is for grant-based sharing (its ADR 0001) |
| Presigned PUT and GET | `bookmarker/backend/api/src/image_access.rs`, `ahara-access/backend/api/src/storage.rs` |
| Frontend login with TOTP | `tastebase/frontend/src/auth.ts` (redirects MFA setup to `ahara-business`; bookmarker's local enrollment is non-compliant) |
| API client, config, lint | `bookmarker/frontend/src/{apiCore,api,config}.ts`, `eslint.config.js` |
| HTTP helpers | shared crate `ahara-infra/crates/ahara-lambda-http` |
| Machine-to-machine client | `ahara/OBSERVABILITY.md`, `ahara-infra/.../services/observability-ingest.tf` (resource server + client-credentials client + SSM) |
| Listener priorities | `ahara/INTEGRATION.md` Step 4 table; 260–261 are free |

## Decisions

Settled:

1. **Storage:** project-owned private S3 bucket `score-shelf-files`
   (versioning, AES256, public access blocked, CORS for the app origin),
   with metadata in the shared RDS database `score_shelf`. Basis:
   bookmarker ADR 0003; the platform has no S3-manifest metadata pattern.
2. **Files never pass through the API.** The ALB WAF caps request bodies at
   8 KB and the Lambda path at about 1 MB, so uploads and downloads use
   presigned S3 URLs and the API handles JSON only.
3. **Publisher identity:** a Cognito resource server `score-shelf` with scope
   `publish` and a client-credentials client, created in `ahara-infra`'s
   services layer because project deployers cannot create resource servers.
   Credentials land in SSM under `/ahara/score-shelf/`.
4. **Authorization in the Lambda:** the ALB checks signatures only. The API
   accepts owner requests from the SPA client's access tokens and publish
   requests from the publisher client with the `score-shelf/publish` scope;
   both client IDs come from the environment.
5. **Hostnames:** `score-shelf.ahara.io` and `api.score-shelf.ahara.io`
   (prefix convention). Listener priorities 260 (`/health`) and 261 (`/*`).
6. **Publish flow:** create version (returns presigned PUTs), upload, commit
   (API checks each object exists with the declared size before marking the
   version ready). Uncommitted versions are invisible to the owner.

Deferred:

- In-browser rendering and MIDI playback: owner decides after using M3.
- Broker secret for the publisher: the owner creates it from the SSM values
  after M0 applies; blocks only the live publish check in M4.

## Milestones

### M0 — Platform registration
Scope: `ahara-infra` control registration (`project-score-shelf.tf`),
database registration in `services/db-migrate.tf`, publisher resource
server and client in `services/score-shelf-publisher.tf`; `ahara`
INTEGRATION.md priority row.
Acceptance: `terraform fmt` and `validate` pass in `ahara-infra`; changes
pushed; CI applies them (checked through the score-shelf repo receiving its
Actions secrets on first CI run, since ahara-infra CI logs are not readable
from this terminal).

### M1 — Project scaffold [depends on M0]
Scope: required files, Makefile, platform.yml, CI, Terraform (context,
cognito app + auth-trigger parameter, bucket, alb-api, website, SSM reads),
backend workspace with `/health`, frontend shell with sign-in.
Acceptance: `make ci` passes locally; `terraform validate` passes.

### M2 — API and data [depends on M1]
Scope: migrations (pieces, versions, files), routes for listing pieces and
versions, publishing and committing versions, presigned downloads, owner
upload of hand edits; authorization split; telemetry operations.
Acceptance: lib tests cover authorization, validation, commit checks and
routing with in-memory doubles; DB tests pass against testcontainers
Postgres; `make ci` passes.

### M3 — Phone UI [depends on M2]
Scope: sign-in with TOTP, piece list, version list with labels and dates,
download links, upload form for hand edits.
Acceptance: vitest covers the API client and views; lint/typecheck pass;
layout works at phone width.

### M4 — Deploy and publish path [depends on M2, M3]
Scope: push to main so CI deploys; `scripts/publish.sh` (token via client
credentials, create, upload, commit) and its use from sigillum-explorations.
Acceptance: `https://api.score-shelf.ahara.io/health` answers; the app loads;
with the owner-created broker secret, Boreal Pocket's current export
publishes and appears in the app.

## Sulion mapping

Root `a93eb01b-b30f-49bf-b4a8-5e3f917c0e3f`. Phases: M0
`a43fcc9e-8158-4dc6-87fb-e84eb12cd925`, M1
`a2021050-0725-4213-a3a2-7d169781ecd9`, M2
`29611e43-6693-4839-a031-909613265b27`, M3
`e7f1c0e8-f5ce-422e-a47a-693a49753104`, M4
`c6a109b2-745a-4c2a-a4df-1f0cdad0bdc3`.

## Current state

- M0: `ahara-infra` 08bb8f0 (registration, `score_shelf` database, publisher
  client) and `ahara` 3a18116 (priorities 260–261) pushed. `terraform
  validate` passed on `ahara-infra`. Its CI result is not visible from this
  terminal (the broker refused the GitHub API call); the first score-shelf
  CI deploy confirms or refutes it.
- M1–M3: implemented. `make ci` passes (34 backend lib tests, 10 frontend
  tests, clippy, telemetry adoption, formatting, terraform fmt);
  `make db-test` passes 5 PostgreSQL tests; `terraform validate` passes;
  `cargo lambda build --release` produces the bootstrap. The phone layout is
  CSS-only and has not been viewed on a device.
- M4 next: push to deploy, check `/health`, then publish Boreal Pocket once
  the owner grants the publisher secret to the broker.
