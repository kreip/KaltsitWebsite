#!/bin/sh
# Make a site user a member of the translators' section with a role, or
# set their role anew. The user must have signed in to the site once.
#
#   ./translators/role.sh <uid> [user|translator|admin]
#
# <uid> is the game uid the user signed in with (`dev-translator` for the
# development sign-in). One role per member (`user` when none is given):
#   user       — sees the section, proposes, agrees, remarks;
#   translator — also approves and rejects, exports for the game;
#   admin      — all of it, "went out in pack <version>", the admin panel.
# The same is done on the site: the admin panel (Admin tab). Prints the member's row.
set -eu
cd "$(dirname "$0")/.."
UID_ARG="${1:?usage: $0 <uid> [user|translator|admin]}"
ROLE="${2:-user}"
case "$ROLE" in
user | translator | admin) ;;
*) echo "role: user, translator or admin" >&2; exit 2 ;;
esac
[ $# -le 2 ] || { echo "one role: user, translator or admin" >&2; exit 2; }
ROLES="$ROLE"
OUT="$(docker compose exec -T postgres sh -c 'psql -v ON_ERROR_STOP=1 -q -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v uid="$1" -v roles="$2"' sh "$UID_ARG" "$ROLES" <<'SQL'
SELECT COUNT(*) AS found FROM users WHERE uid = :'uid' \gset
\if :found
INSERT INTO translators.members (user_id, note, roles)
SELECT id, 'translators/role.sh', string_to_array(:'roles', ',') FROM users WHERE uid = :'uid'
ON CONFLICT (user_id) DO UPDATE SET roles = EXCLUDED.roles;
SELECT u.uid, COALESCE(u.nickname, '') AS nickname, array_to_string(m.roles, ', ') AS roles, m.added_at
FROM translators.members m JOIN users u ON u.id = m.user_id WHERE u.uid = :'uid';
\else
\echo 'NO_SUCH_USER'
\endif
SQL
)"
case "$OUT" in
*NO_SUCH_USER*) echo "No such user: they have to sign in to the site once." >&2; exit 1 ;;
esac
echo "$OUT"
