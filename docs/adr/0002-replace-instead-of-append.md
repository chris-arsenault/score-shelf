# 0002 — Replace Instead of Append for Non-Meaningful Revisions

- Status: Accepted
- Date: 2026-09-27

## Context

Every publish created a new numbered version. Within four days Boreal Pocket
had six versions, two of which were not revisions of the music: an export fix
for the bass drum's notation-program import, and a re-publish of the same
music tagged with its commit instead of "uncommitted". The owner wants the
shelf to hold only meaningful revisions: fixes of that kind should replace
the previous version. Failing that, versions should be marked major or minor,
with only major versions shown by default.

## Decision

A publish may replace the piece's latest ready version. The replacement is a
new version row that takes over the number; when it commits, the old row
becomes `replaced` in the same transaction. A version may also be retired
(hidden). The publisher may replace or retire only agent versions. Numbers are
unique among ready versions. A new version is numbered after the highest ready
or pending number, so retiring the latest version frees its number.

Replaced and retired rows keep their metadata and S3 objects. They are never
listed or downloadable.

## Alternatives considered

- **Major/minor flag with a UI filter**: every fix would still be a
  separate version with its own number, so the history would still carry
  them, and every client would need the filter. The owner named this as the
  fallback if replacing were impossible; it is not.
- **Overwrite the files of the existing row**: breaks the rule that a
  version's files are immutable once committed. A failed upload would also
  leave a half-replaced version visible.
- **Delete replaced rows and objects**: loses history. It would also need S3
  delete permission for the Lambda, which it does not have.

## Consequences

- Clients choose between a revision (new number) and a fix (`--replace`).
- A version number can later name different files (after a replace or a
  retire-and-republish), so the number and label identify a revision and the
  file ids identify bytes.
- Hidden rows and objects accumulate. The volume is a few megabytes per fix.
