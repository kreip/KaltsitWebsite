/**
 * Whether the header shows the translators' section («Русификатор»): to every
 * signed-in user. A member works there; anyone else reads the game's texts
 * and may request access (the page tells them apart through `/translators/me`).
 * The name is kept from when the item was for members only.
 */
import { useAuth } from "#/hooks/use-auth";

export function useTranslatorsMember(): boolean {
    const { user } = useAuth();
    return Boolean(user);
}
