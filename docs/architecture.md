# Architecture

Score Shelf keeps numbered versions of composition exports (MusicXML, MIDI,
optionally audio or PDF) for one owner. The composition agent publishes new
versions from the dev box; the owner downloads them and uploads hand edits.

## Components

| Component                   | Responsibility                                                                 |
| --------------------------- | ------------------------------------------------------------------------------ |
| `backend/api`               | Rust HTTP Lambda: auth decisions, validation, PostgreSQL store, S3 presigning   |
| `frontend/`                 | Vite React SPA: sign-in with TOTP, piece list, versions, downloads, uploads     |
| `db/migrations/`            | `pieces`, `versions`, `version_files`                                           |
| `infrastructure/terraform/` | Website, API, Cognito app client, private bucket, runtime config, alarms        |
| `scripts/shelf.sh`          | Publisher client: client-credentials token, publish (or replace), retire, list, pull |

## Platform integration

The API runs behind the shared Ahara ALB through the `alb-api` module on
listener priorities 260 (`GET /health`, public) and 261 (everything else,
JWT-validated). The website module serves the SPA at `score-shelf.ahara.io`
with runtime config in `/config.js`. Terraform reads the shared VPC, ALB,
Cognito and RDS through `platform-context`; the database credentials come
from `/ahara/db/score-shelf/*`.

`ahara-infra` owns three things for this project: the deployer registration
(`control/project-score-shelf.tf`), the `score_shelf` database registration,
and the publisher identity (`services/score-shelf-publisher.tf`): a Cognito
resource server `score-shelf` with scope `publish`, a client-credentials
client, and its id, secret, scope and token URL in `/ahara/score-shelf/`.

## Who may call the API

The ALB validates the signature and issuer of any token from the shared pool,
so the API decides:

| Token | Principal | Version source |
| --- | --- | --- |
| Access token from the `score-shelf-app` client | owner | `owner` |
| Access token from the publisher client with `score-shelf/publish` scope | publisher | `agent` |
| Anything else (ID tokens, other apps' clients, missing scope) | rejected (401/403) | — |

Sign-in to the app client is gated by the platform pre-auth trigger
(`/ahara/auth-trigger/clients/score-shelf-app`).

## Publishing a version

```text
POST /pieces/{slug}/versions        -> version_id, number, presigned PUT per file
PUT  <presigned url>                -> S3 (the API never sees the bytes)
POST /versions/{version_id}/commit  -> API HEADs each object, checks size, marks ready
```

Pieces are created on first publish. Version numbers count per piece and are
assigned under a row lock. Pending versions are invisible; a commit fails if
any object is missing or has the wrong size, and succeeds once.

The shelf holds meaningful revisions only ([ADR 0002](adr/0002-replace-instead-of-append.md)):

```text
POST /pieces/{slug}/versions {"replaces": N, ...}  -> a pending version numbered N
POST /versions/{version_id}/commit                  -> N's old row becomes 'replaced'
POST /pieces/{slug}/versions/{number}/retire        -> the version becomes 'retired'
```

A replacement must name the piece's latest ready version and come from the
same source (the publisher cannot replace a hand edit). The old version stays
visible until the replacement commits; both status changes happen in one
transaction. A retired version is hidden. The owner may retire any version,
the publisher only agent versions. A new version is numbered after the highest
ready or pending number, so retiring the latest lets the next publish reuse
its number. Replaced and retired rows keep their files and history in the
database and bucket but are never listed or downloadable. Numbers are unique
among ready versions.

## Reading

| Route | Returns |
| --- | --- |
| `GET /pieces` | Pieces with at least one ready version, newest activity first, with the latest version |
| `GET /pieces/{slug}` | The piece and all ready versions with their files |
| `GET /files/{id}/download` | A 10-minute presigned GET with `Content-Disposition: attachment` |
| `GET /me` | The caller's principal kind |

Objects are stored at `pieces/{slug}/v{number}/{file_id}/{filename}` in a
versioned, AES256-encrypted bucket with public access blocked. The bucket's
CORS rule allows PUT/GET/HEAD from the app origin and `localhost:5173`.
