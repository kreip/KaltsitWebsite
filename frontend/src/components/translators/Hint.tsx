/**
 * The site's hint on any element of the section: the kit Tooltip (a frame,
 * padding, plain text; opens on a tap when the element has no action of its
 * own), never the browser's `title`. A long text wraps at a readable width.
 */
import type React from "react";
import { Tooltip, TooltipPopup, TooltipTrigger } from "#/components/ui/tooltip";

export function Hint({ text, children }: { text: React.ReactNode; children: React.ReactElement }): React.ReactElement {
    if (text === null || text === undefined || text === "") return children;
    return (
        <Tooltip>
            <TooltipTrigger render={children} />
            <TooltipPopup className="max-w-xs whitespace-pre-line">{text}</TooltipPopup>
        </Tooltip>
    );
}
