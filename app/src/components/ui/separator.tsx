/**
 * Visual separator component for dividing content sections
 *
 * @cognitive-load 0/10 - Passive visual element, no cognitive processing required
 * @attention-economics Neutral structure: creates visual boundaries without demanding attention
 * @trust-building Consistent spacing, clear content grouping, predictable visual hierarchy
 * @accessibility role="separator" or role="none" for decorative, orientation for screen readers
 * @semantic-meaning Visual division: horizontal=between sections, vertical=between inline items
 *
 * @usage-patterns
 * DO: Use to group related content visually
 * DO: Use horizontal for section breaks
 * DO: Use vertical for inline item separation (toolbars, menus)
 * DO: Set decorative=true when purely visual
 * NEVER: Overuse separators, use when whitespace alone suffices
 *
 * @example
 * ```tsx
 * // Horizontal section divider
 * <Separator />
 *
 * // Vertical toolbar divider
 * <Separator orientation="vertical" className="h-4" />
 * ```
 */
import * as React from 'react';
import classy from '@/src/lib/primitives/classy';

export interface SeparatorProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Orientation of the separator */
  orientation?: 'horizontal' | 'vertical';
  /** Whether the separator is purely decorative */
  decorative?: boolean;
}

export const Separator = React.forwardRef<HTMLDivElement, SeparatorProps>(
  ({ className, orientation = 'horizontal', decorative = true, ...props }, ref) => {
    const orientationClasses = {
      horizontal: 'h-px w-full',
      vertical: 'h-full w-px',
    };

    return (
      // biome-ignore lint/a11y/useAriaPropsSupportedByRole: aria-orientation is valid when role="separator"
      <div
        ref={ref}
        role={decorative ? 'none' : 'separator'}
        aria-orientation={!decorative ? orientation : undefined}
        className={classy('shrink-0 bg-border', orientationClasses[orientation], className)}
        {...props}
      />
    );
  },
);

Separator.displayName = 'Separator';
