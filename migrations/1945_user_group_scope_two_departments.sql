-- The department scope of a permission group exists twice: one list for `Personal`
-- and one for `Personal-Disposition`, so the two areas do not influence each other.
-- The single list of 1944 becomes the list of `Personal`.
UPDATE "where_user_group_scope"
SET "kind" = 'personal-department'
WHERE "kind" = 'department';

COMMENT ON COLUMN "where_user_group_scope"."kind" IS
    'Art der Zuordnung: mandant, lager, personal-department, personaldispo-department oder client.';
