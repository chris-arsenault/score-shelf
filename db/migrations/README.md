# Database Migrations

Platform PostgreSQL migrations for the `score_shelf` database.

Migration files follow the Ahara layout:

```text
db/migrations/001_create_shelf.sql
db/migrations/rollback/001_create_shelf.sql
```

The model holds pieces, their numbered versions (pending until committed,
then ready), and the files each version holds. File bytes live in the
project's private S3 bucket; `version_files.object_key` points at them.
Rollback files drop only project-owned objects in reverse dependency order.

`backend/api/tests/store_pg.rs` applies this migration against PostgreSQL in
a container, exercises the store, and applies the rollback (`make db-test`).

Do not create database roles, users, grants, default privileges, or databases in
project migrations. The Ahara migration service owns those platform concerns.
