/**
 * Semantic container component for layout structure and content boundaries
 *
 * @cognitive-load 0/10 - Invisible structure that reduces visual complexity
 * @attention-economics Neutral structural element: Controls content width and breathing room without competing for attention
 * @trust-building Predictable boundaries and consistent spacing patterns
 * @accessibility Semantic HTML elements with proper landmark roles for screen readers
 * @semantic-meaning Element-driven behavior: main=primary landmark, section=structural grouping, article=readable content with typography, aside=supplementary, div=no semantics
 *
 * @usage-patterns
 * DO: Use main for the primary content area (once per page)
 * DO: Use section for structural grouping within grids
 * DO: Use article for readable content - typography is automatic
 * DO: Use aside for supplementary content, add surface classes for depth
 * DO: Spacing happens inside (padding), not outside (no margins)
 * NEVER: Nest containers unnecessarily
 * NEVER: Use margins for spacing - let parent Grid handle gaps
 * NEVER: Use @container without w-full in flex/grid contexts (causes width collapse in Tailwind v4)
 *
 * @example
 * ```tsx
 * <Container as="main" size="6xl" padding="6">
 *   <Container as="article">
 *     <h1>Title</h1>
 *     <p>Typography just works.</p>
 *   </Container>
 * </Container>
 * ```
 */
import * as React from 'react';
import classy from '@/src/lib/primitives/classy';

type ContainerElement = 'div' | 'main' | 'section' | 'article' | 'aside';

/** Background preset options for containers */
export type ContainerBackground = 'none' | 'muted' | 'accent' | 'card';

export interface ContainerProps extends React.HTMLAttributes<HTMLElement> {
  /** Semantic element - determines behavior and accessibility role */
  as?: ContainerElement;

  /**
   * Max-width constraint using Tailwind sizing
   * @default 'full' for main, undefined for others
   */
  size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | '5xl' | '6xl' | '7xl' | 'full';

  /**
   * Internal padding using Tailwind spacing scale
   * Spacing happens inside containers, not via margins
   */
  padding?: '0' | '1' | '2' | '3' | '4' | '5' | '6' | '8' | '10' | '12' | '16' | '20' | '24';

  /**
   * Vertical flow gap between children.
   * When true, derives gap from size by walking the spacing scale positions.
   * When a spacing value, overrides the size-derived default.
   * Applies flex flex-col gap-{n} to create a vertical stack.
   */
  gap?: boolean | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '8' | '10' | '12' | '16' | '20' | '24';

  /**
   * Enable container queries on this element
   * Children can use @container queries to respond to this container's size
   * @default true
   */
  query?: boolean;

  /**
   * Container query name for targeted queries
   * Use with @container/name in child styles
   */
  queryName?: string;

  // ============================================================================
  // Editable Props (R-202)
  // ============================================================================

  /**
   * Enable editing mode for block editor
   * Shows dashed outline and enables drop zone
   */
  editable?: boolean | undefined;

  /**
   * Background color preset
   * Allowed presets: 'none', 'muted', 'accent', 'card'
   */
  background?: ContainerBackground | undefined;

  /**
   * Show drop zone indicator for child blocks
   * Displays placeholder when container is empty in edit mode
   */
  showDropZone?: boolean | undefined;

  /**
   * Called when background changes in edit mode
   */
  onBackgroundChange?: ((background: ContainerBackground) => void) | undefined;
}

const sizeClasses: Record<string, string> = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-xl',
  '2xl': 'max-w-2xl',
  '3xl': 'max-w-3xl',
  '4xl': 'max-w-4xl',
  '5xl': 'max-w-5xl',
  '6xl': 'max-w-6xl',
  '7xl': 'max-w-7xl',
  full: 'w-full',
};

const paddingClasses: Record<string, string> = {
  '0': 'p-0',
  '1': 'p-1',
  '2': 'p-2',
  '3': 'p-3',
  '4': 'p-4',
  '5': 'p-5',
  '6': 'p-6',
  '8': 'p-8',
  '10': 'p-10',
  '12': 'p-12',
  '16': 'p-16',
  '20': 'p-20',
  '24': 'p-24',
};

const gapClasses: Record<string, string> = {
  '0': 'flex flex-col gap-0',
  '1': 'flex flex-col gap-1',
  '2': 'flex flex-col gap-2',
  '3': 'flex flex-col gap-3',
  '4': 'flex flex-col gap-4',
  '5': 'flex flex-col gap-5',
  '6': 'flex flex-col gap-6',
  '8': 'flex flex-col gap-8',
  '10': 'flex flex-col gap-10',
  '12': 'flex flex-col gap-12',
  '16': 'flex flex-col gap-16',
  '20': 'flex flex-col gap-20',
  '24': 'flex flex-col gap-24',
};

/**
 * Size-to-gap mapping: walks through the spacing scale positions
 * from the component-padding tier (3-4) into the section-padding tier (5-12).
 * These are spacing SCALE POSITIONS, not pixel values -- Tailwind v4 resolves
 * gap-N to calc(var(--spacing) * N), so actual values track the design system's
 * baseSpacingUnit automatically.
 */
const sizeGapScale: Record<string, string> = {
  sm: '3',
  md: '4',
  lg: '5',
  xl: '6',
  '2xl': '6',
  '3xl': '8',
  '4xl': '8',
  '5xl': '10',
  '6xl': '10',
  '7xl': '12',
};

// Article typography - the magic for readable content
const articleTypography = [
  // Base prose styling
  '[&_p]:leading-relaxed',
  '[&_p]:mb-4',
  '[&_p:last-child]:mb-0',

  // Headings
  '[&_h1]:text-4xl [&_h1]:font-bold [&_h1]:tracking-tight [&_h1]:mb-4 [&_h1]:mt-0 [&_h1]:text-accent-foreground',
  '[&_h2]:text-3xl [&_h2]:font-semibold [&_h2]:tracking-tight [&_h2]:mb-3 [&_h2]:mt-8 [&_h2]:first:mt-0 [&_h2]:text-accent-foreground',
  '[&_h3]:text-2xl [&_h3]:font-semibold [&_h3]:mb-2 [&_h3]:mt-6 [&_h3]:text-accent-foreground',
  '[&_h4]:text-xl [&_h4]:font-semibold [&_h4]:mb-2 [&_h4]:mt-4 [&_h4]:text-accent-foreground',

  // Lists
  '[&_ul]:list-disc [&_ul]:pl-6 [&_ul]:mb-4',
  '[&_ol]:list-decimal [&_ol]:pl-6 [&_ol]:mb-4',
  '[&_li]:mb-1',

  // Links
  '[&_a]:text-primary [&_a]:underline [&_a]:underline-offset-4 [&_a:hover]:text-primary/80',

  // Blockquotes
  '[&_blockquote]:border-l-4 [&_blockquote]:border-muted [&_blockquote]:pl-4 [&_blockquote]:italic [&_blockquote]:my-4',

  // Code
  '[&_code]:bg-muted [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-sm [&_code]:font-mono',
  '[&_pre]:bg-muted [&_pre]:p-4 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-4',
  '[&_pre_code]:bg-transparent [&_pre_code]:p-0',

  // Horizontal rules
  '[&_hr]:border-border [&_hr]:my-8',

  // Images
  '[&_img]:rounded-lg [&_img]:my-4',

  // Tables
  '[&_table]:w-full [&_table]:my-4',
  '[&_th]:border [&_th]:border-border [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:font-semibold',
  '[&_td]:border [&_td]:border-border [&_td]:px-3 [&_td]:py-2',

  // Optimal reading width
  'max-w-prose',
].join(' ');

/** Background class mapping */
const backgroundClasses: Record<ContainerBackground, string> = {
  none: '',
  muted: 'bg-muted',
  accent: 'bg-accent',
  card: 'bg-card',
};

/**
 * Drop zone placeholder for empty containers in edit mode
 */
function DropZonePlaceholder() {
  return (
    <div className="flex min-h-24 items-center justify-center rounded-md border-2 border-dashed border-muted-foreground/25 bg-muted/30 p-4 text-muted-foreground">
      <span className="text-sm">Drop blocks here</span>
    </div>
  );
}

export const Container = React.forwardRef<HTMLElement, ContainerProps>(
  (
    {
      as: Element = 'div',
      size,
      padding,
      gap,
      query = true,
      queryName,
      editable,
      background,
      showDropZone,
      onBackgroundChange: _onBackgroundChange,
      className,
      style,
      children,
      ...props
    },
    ref,
  ) => {
    // TODO: Implement background picker UI that calls _onBackgroundChange
    void _onBackgroundChange;

    const isArticle = Element === 'article';
    const isEmpty = React.Children.count(children) === 0;

    const resolvedGap = gap === true ? (size && sizeGapScale[size]) || '6' : gap || undefined;

    const classes = classy(
      // Container queries - w-full prevents width collapse when container-type: inline-size
      // is applied to flex/grid children (Tailwind v4 behavior)
      query && '@container w-full',

      // Size constraint
      size && sizeClasses[size],

      // Centering for sized containers
      size && size !== 'full' && 'mx-auto',

      // Padding
      padding && paddingClasses[padding],

      // Vertical flow with gap
      resolvedGap && gapClasses[resolvedGap],

      // Background (R-202)
      background && backgroundClasses[background],

      // Article gets typography
      isArticle && articleTypography,

      // Editable mode styling (R-202)
      editable && 'outline-2 outline-dashed outline-muted-foreground/30 outline-offset-2 rounded',

      // User classes
      className,
    );

    // Container query name via style
    const containerStyle: React.CSSProperties = {
      ...style,
      ...(queryName && { containerName: queryName }),
    };

    // Determine content to render
    const content = editable && showDropZone && isEmpty ? <DropZonePlaceholder /> : children;

    return React.createElement(
      Element,
      {
        ref,
        className: classes || undefined,
        style: Object.keys(containerStyle).length > 0 ? containerStyle : undefined,
        'data-editable': editable || undefined,
        'data-background': background || undefined,
        ...props,
      },
      content,
    );
  },
);

Container.displayName = 'Container';

export default Container;
