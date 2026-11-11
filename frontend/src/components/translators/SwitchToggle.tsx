/** A switch with its label and a one-line hint, above a table of the section. */
import type React from "react";
import { Switch } from "#/components/ui/switch";

export function Toggle({ id, checked, onChange, label, hint }: { id: string; checked: boolean; onChange: (checked: boolean) => void; label: string; hint: string }): React.ReactElement {
    return (
        <label htmlFor={id} className="flex items-center gap-3">
            <Switch id={id} checked={checked} onCheckedChange={onChange} />
            <span className="flex flex-col gap-0.5">
                <span className="font-medium text-sm">{label}</span>
                <span className="text-muted-foreground text-xs">{hint}</span>
            </span>
        </label>
    );
}
