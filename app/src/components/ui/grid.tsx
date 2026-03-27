/**
 * Intelligent layout grid with semantic presets and embedded design reasoning
 *
 * @cognitive-load 4/10 - Layout container with intelligent presets that respect Miller's Law
 * @attention-economics Preset hierarchy: linear=democratic attention, golden=hierarchical flow, bento=complex attention patterns
 * @trust-building Mathematical spacing, Miller's Law cognitive load limits, consistent preset behavior
 * @accessibility WCAG AAA compliance with optional ARIA grid role for interactive layouts
 * @semantic-meaning Layout intelligence: linear=equal-priority content, golden=natural hierarchy, bento=content showcases with semantic asymmetry
 *
 * @usage-patterns
 * DO: Linear - Product catalogs, image galleries, equal-priority content
 * DO: Golden - Editorial layouts, feature showcases, natural hierarchy
 * DO: Bento - Dashboards, content showcases (use sparingly, high cognitive load)
 * DO: Limit items to 8 max on wide screens (Miller's Law)
 * NEVER: Decorative asymmetry without semantic meaning
 * NEVER: Exceed cognitive load limits
 *
 * @example
 * ```tsx
 * // Equal-priority grid
 * <Grid preset="linear" columns={3} gap="4">
 *   <Grid.Item>Card 1</Grid.Item>
 *   <Grid.Item>Card 2</Grid.Item>
 *   <Grid.Item>Card 3</Grid.Item>
 * </Grid>
 *
 * // Bento dashboard layout
 * <Grid preset="bento" pattern="dashboard">
 *   <Grid.Item priority="primary">Main Metric</Grid.Item>
 *   <Grid.Item priority="secondary">Chart</Grid.Item>
 * </Grid>
 * ```
 */
import * as React from 'react';
import classy from '@/src/lib/primitives/classy';

// ==================== Types ====================

type BentoPattern = 'editorial' | 'dashboard' | 'feature' | 'portfolio';
type GridPreset = 'linear' | 'golden' | 'bento';
type ContentPriority = 'primary' | 'secondary' | 'tertiary';

// ==================== Context ====================

interface GridContextValue {
  preset: GridPreset;
  pattern: BentoPattern | undefined;
  editable: boolean | undefined;
  showColumnDropZones: boolean | undefined;
}

const GridContext = React.createContext<GridContextValue | null>(null);

function useGridContext() {
  return React.useContext(GridContext);
}

// ==================== Grid ====================

/** Grid configuration for onConfigChange callback */
export interface GridConfig {
  columns?: 1 | 2 | 3 | 4 | 5 | 6 | 'auto';
  gap?: '0' | '1' | '2' | '3' | '4' | '5' | '6' | '8' | '10' | '12';
  preset?: GridPreset;
}

export interface GridProps extends React.HTMLAttributes<HTMLDivElement> {
  /**
   * Semantic preset with embedded UX reasoning
   * - linear: Democratic attention, equal columns (cognitive load: 2/10)
   * - golden: Hierarchical attention, 2:1 ratio (cognitive load: 4/10)
   * - bento: Complex attention, asymmetric (cognitive load: 6/10)
   * @default 'linear'
   */
  preset?: GridPreset;

  /**
   * Bento layout pattern - only applies when preset="bento"
   * - editorial: Hero + supporting articles
   * - dashboard: Primary metric + supporting data
   * - feature: Main feature + benefits
   * - portfolio: Featured work + gallery
   */
  pattern?: BentoPattern;

  /**
   * Column count for linear preset
   * Responsive object or single value
   * @default auto-fit based on content
   */
  columns?: 1 | 2 | 3 | 4 | 5 | 6 | 'auto';

  /**
   * Gap between items using Tailwind spacing
   */
  gap?: '0' | '1' | '2' | '3' | '4' | '5' | '6' | '8' | '10' | '12';

  /**
   * Accessibility role
   * - 'presentation': Layout-only (default)
   * - 'grid': Interactive grid with keyboard navigation
   */
  role?: 'presentation' | 'grid';

  /**
   * Accessible label - required when role="grid"
   */
  'aria-label'?: string;

  // ============================================================================
  // Editable Props (R-202)
  // ============================================================================

  /**
   * Enable editing mode for block editor
   * Shows column guides and enables drop zones
   */
  editable?: boolean | undefined;

  /**
   * Show drop zone indicators in each column
   * Displays placeholder when cells are empty in edit mode
   */
  showColumnDropZones?: boolean | undefined;

  /**
   * Called when grid configuration changes in edit mode
   */
  onConfigChange?: ((config: GridConfig) => void) | undefined;
}

const gapClasses: Record<string, string> = {
  '0': 'gap-0',
  '1': 'gap-1',
  '2': 'gap-2',
  '3': 'gap-3',
  '4': 'gap-4',
  '5': 'gap-5',
  '6': 'gap-6',
  '8': 'gap-8',
  '10': 'gap-10',
  '12': 'gap-12',
};

const columnClasses: Record<string | number, string> = {
  1: 'grid-cols-1',
  2: 'grid-cols-2',
  3: 'grid-cols-3',
  4: 'grid-cols-4',
  5: 'grid-cols-5',
  6: 'grid-cols-6',
  auto: 'grid-cols-1 @sm:grid-cols-2 @lg:grid-cols-3 @xl:grid-cols-4',
};

// Bento pattern grid definitions
const bentoPatterns: Record<BentoPattern, string> = {
  // Hero (2x2) + 2 side items
  editorial: 'grid-cols-3 grid-rows-2 [&>*:first-child]:col-span-2 [&>*:first-child]:row-span-2',
  // Primary metric large + supporting smaller
  dashboard: 'grid-cols-4 grid-rows-2 [&>*:first-child]:col-span-2 [&>*:first-child]:row-span-2',
  // Feature left + benefits right
  feature: 'grid-cols-2 [&>*:first-child]:row-span-2',
  // Featured large + gallery grid
  portfolio: 'grid-cols-3 grid-rows-3 [&>*:first-child]:col-span-2 [&>*:first-child]:row-span-2',
};

// Golden ratio: approximately 1.618:1, we use 2:1 for grid simplicity
const goldenClasses = 'grid-cols-3 [&>*:first-child]:col-span-2';

function GridRoot({
  preset = 'linear',
  pattern,
  columns = 'auto',
  gap = '4',
  role = 'presentation',
  editable,
  showColumnDropZones,
  onConfigChange: _onConfigChange,
  className,
  children,
  ...props
}: GridProps) {
  // TODO: Implement grid config UI that calls _onConfigChange
  void _onConfigChange;
  const classes = classy(
    'grid',

    // Gap
    gap && gapClasses[gap],

    // Preset-specific layouts
    preset === 'linear' && columnClasses[columns],
    preset === 'golden' && goldenClasses,
    preset === 'bento' && pattern && bentoPatterns[pattern],

    // Responsive defaults for linear
    preset === 'linear' && columns === 'auto' && 'sm:grid-cols-2 lg:grid-cols-3',

    // Editable mode styling (R-202)
    editable && 'outline-2 outline-dashed outline-muted-foreground/30 outline-offset-2 rounded p-2',

    className,
  );

  const contextValue: GridContextValue = { preset, pattern, editable, showColumnDropZones };

  return (
    <GridContext.Provider value={contextValue}>
      <div
        role={role === 'grid' ? 'grid' : undefined}
        className={classes}
        data-editable={editable || undefined}
        data-preset={preset}
        data-columns={typeof columns === 'number' ? columns : undefined}
        {...props}
      >
        {children}
      </div>
    </GridContext.Provider>
  );
}

// ==================== Grid.Item ====================

export interface GridItemProps extends React.HTMLAttributes<HTMLDivElement> {
  /**
   * Content priority for bento layouts
   * Affects visual hierarchy and grid placement
   */
  priority?: ContentPriority;

  /**
   * Explicit column span override
   */
  colSpan?: 1 | 2 | 3 | 4;

  /**
   * Explicit row span override
   */
  rowSpan?: 1 | 2 | 3;
}

const colSpanClasses: Record<number, string> = {
  1: 'col-span-1',
  2: 'col-span-2',
  3: 'col-span-3',
  4: 'col-span-4',
};

const rowSpanClasses: Record<number, string> = {
  1: 'row-span-1',
  2: 'row-span-2',
  3: 'row-span-3',
};

/**
 * Drop zone placeholder for empty grid items in edit mode
 */
function GridItemDropZone() {
  return (
    <div className="flex min-h-16 items-center justify-center rounded border-2 border-dashed border-muted-foreground/20 bg-muted/20 p-2 text-muted-foreground">
      <span className="text-xs">Drop here</span>
    </div>
  );
}

function GridItem({ priority, colSpan, rowSpan, className, children, ...props }: GridItemProps) {
  const context = useGridContext();
  const isEmpty = React.Children.count(children) === 0;

  const classes = classy(
    // Explicit spans override preset behavior
    colSpan && colSpanClasses[colSpan],
    rowSpan && rowSpanClasses[rowSpan],

    // Editable mode styling (R-202)
    context?.editable && 'outline outline-1 outline-dashed outline-muted-foreground/20 rounded',

    className,
  );

  // Show drop zone in edit mode when item is empty
  const content =
    context?.editable && context?.showColumnDropZones && isEmpty ? <GridItemDropZone /> : children;

  return (
    <div className={classes || undefined} data-priority={priority} {...props}>
      {content}
    </div>
  );
}

// ==================== Display Names ====================

GridRoot.displayName = 'Grid';
GridItem.displayName = 'GridItem';

// ==================== Compound Export ====================

export const Grid = Object.assign(GridRoot, {
  Item: GridItem,
});

export default Grid;
