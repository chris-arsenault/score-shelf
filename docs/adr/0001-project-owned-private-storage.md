# 0001 — Project-Owned Private Storage With a Separate Publisher Identity

- Status: Accepted
- Date: 2026-09-23

## Context

Score Shelf stores composition exports for one owner and receives new
versions from an agent on the dev box. The files are private. The platform
offers `ahara-access` for grant-based sharing with other principals, and
Linkdrop has already decided the single-owner case
(`bookmarker/docs/adr/0003-project-owned-snapshot-storage.md`).

The shared ALB's WAF limits request bodies to 8 KB and the Lambda path to
about 1 MB, so score files cannot pass through the API. Project deployers
cannot create Cognito resource servers.

## Decision

Score Shelf stores files in a project-owned private S3 bucket and keeps piece,
version and file metadata in its PostgreSQL database. Clients move bytes with
presigned URLs. The agent authenticates with a client-credentials client and
scope `score-shelf/publish` defined in `ahara-infra`; the API authorizes by
client id and scope because the ALB checks signatures only.

## Alternatives considered

- **`ahara-access` assets** — built for sharing with other principals; each
  publish would need principal, resource, upload and grant calls, it has no
  version model, and its bucket CORS is scoped to another app.
- **S3-only metadata (manifests and listings)** — fewer moving parts, but no
  platform precedent, and commit/visibility rules would live in object
  conventions rather than constraints.
- **Committing exports to Git** — rejected by the owner as the wrong tool for
  build artifacts.
- **Resource server in this repo** — needs `cognito-idp:*ResourceServer` for
  every project deployer.

## Consequences

Terraform creates the bucket with versioning, encryption, public access
blocked and a CORS rule for the app origin. The API owns commit and
visibility semantics. Sharing a piece with anyone else later would go
through `ahara-access` rather than widening this API.
